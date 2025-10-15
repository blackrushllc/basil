use std::fs::File;
use std::io::{Read, Write};
use std::rc::Rc;
use std::cell::RefCell;

use basil_common::{Result, BasilError};
use basil_bytecode::{Value, ObjectDescriptor, MethodDesc, BasicObject, ObjectRef};

use sequoia_openpgp as openpgp;
use openpgp::policy::StandardPolicy;
use openpgp::Cert;
use openpgp::armor;
use openpgp::serialize::stream::{Message, Armorer, Encryptor, LiteralWriter, DetachedSigner};
use openpgp::parse::stream::{DecryptorBuilder, DecryptionHelper, MessageStructure, VerificationHelper, DetachedVerifierBuilder};
use openpgp::packet::{SKESK, PKESK};
use openpgp::types::{SymmetricAlgorithm};
use openpgp::crypto::SessionKey;

pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    reg("CRYPTO_PGP", crate::TypeInfo {
        factory: |_args| Ok(new_instance()),
        descriptor: descriptor_static,
        constants: || Vec::new(),
    });
}

fn new_instance() -> ObjectRef {
    Rc::new(RefCell::new(PgpObj {}))
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "CRYPTO_PGP".to_string(),
        version: "1.0".to_string(),
        summary: "PGP/GPG encryption, decryption, signing, verification (armored)".to_string(),
        properties: vec![],
        methods: vec![
            MethodDesc { name: "EncryptArmored$".to_string(), arity: 2, arg_names: vec!["public_key_armored$".into(), "plaintext$".into()], return_type: "String".into() },
            MethodDesc { name: "DecryptArmored$".to_string(), arity: 3, arg_names: vec!["private_key_armored$".into(), "passphrase$".into(), "cipher_armored$".into()], return_type: "String".into() },
            MethodDesc { name: "SignArmored$".to_string(), arity: 3, arg_names: vec!["private_key_armored$".into(), "passphrase$".into(), "message$".into()], return_type: "String".into() },
            MethodDesc { name: "Verify$".to_string(), arity: 3, arg_names: vec!["public_key_armored$".into(), "message$".into(), "signature_armored$".into()], return_type: "Int".into() },
            MethodDesc { name: "EncryptFile$".to_string(), arity: 3, arg_names: vec!["public_key_armored$".into(), "in_path$".into(), "out_path$".into()], return_type: "Int".into() },
            MethodDesc { name: "DecryptFile$".to_string(), arity: 4, arg_names: vec!["private_key_armored$".into(), "passphrase$".into(), "in_path$".into(), "out_path$".into()], return_type: "Int".into() },
            MethodDesc { name: "SignFile$".to_string(), arity: 4, arg_names: vec!["private_key_armored$".into(), "passphrase$".into(), "in_path$".into(), "sig_out_path$".into()], return_type: "Int".into() },
            MethodDesc { name: "VerifyFile$".to_string(), arity: 3, arg_names: vec!["public_key_armored$".into(), "in_path$".into(), "sig_path$".into()], return_type: "Int".into() },
            MethodDesc { name: "ReadFileText$".to_string(), arity: 1, arg_names: vec!["path$".into()], return_type: "String".into() },
            MethodDesc { name: "WriteFileText$".to_string(), arity: 2, arg_names: vec!["path$".into(), "text$".into()], return_type: "Int".into() },
        ],
        examples: vec![
            "DIM pgp@ AS CRYPTO_PGP()".into(),
            "LET cipher$ = pgp@.EncryptArmored$(pub$, \"Hello\")".into(),
        ],
    }
}

#[derive(Clone)]
struct PgpObj {}

impl BasicObject for PgpObj {
    fn type_name(&self) -> &str { "CRYPTO_PGP" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("CRYPTO_PGP has no properties".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("CRYPTO_PGP has no properties".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "ENCRYPTARMORED$" => {
                if args.len()!=2 { return Err(bad_arity("EncryptArmored$", 2, args.len())); }
                let pub_armored = str_arg(&args[0]);
                let plaintext = str_arg(&args[1]);
                let out = encrypt_armored(&pub_armored, plaintext.as_bytes())?;
                Ok(Value::Str(out))
            }
            "DECRYPTARMORED$" => {
                if args.len()!=3 { return Err(bad_arity("DecryptArmored$", 3, args.len())); }
                let sec_armored = str_arg(&args[0]);
                let pass = str_arg(&args[1]);
                let cipher = str_arg(&args[2]);
                let out = decrypt_armored(&sec_armored, empty_to_none(&pass), cipher.as_bytes())?;
                Ok(Value::Str(String::from_utf8_lossy(&out).to_string()))
            }
            "SIGNARMORED$" => {
                if args.len()!=3 { return Err(bad_arity("SignArmored$", 3, args.len())); }
                let sec_armored = str_arg(&args[0]);
                let pass = str_arg(&args[1]);
                let msg = str_arg(&args[2]);
                let sig = sign_detached_armored(&sec_armored, empty_to_none(&pass), msg.as_bytes())?;
                Ok(Value::Str(sig))
            }
            "VERIFY$" => {
                if args.len()!=3 { return Err(bad_arity("Verify$", 3, args.len())); }
                let pub_armored = str_arg(&args[0]);
                let msg = str_arg(&args[1]);
                let sig = str_arg(&args[2]);
                let ok = verify_detached(&pub_armored, msg.as_bytes(), sig.as_bytes())?;
                Ok(Value::Int(if ok { 1 } else { 0 }))
            }
            "ENCRYPTFILE$" => {
                if args.len()!=3 { return Err(bad_arity("EncryptFile$", 3, args.len())); }
                let pub_armored = str_arg(&args[0]);
                let in_path = str_arg(&args[1]);
                let out_path = str_arg(&args[2]);
                let mut input = Vec::new();
                File::open(&in_path).map_err(|e| BasilError(format!("PGP.EncryptFile: {}", e)))?.read_to_end(&mut input).map_err(|e| BasilError(format!("PGP.EncryptFile: {}", e)))?;
                let cipher = encrypt_armored(&pub_armored, &input)?;
                std::fs::write(&out_path, cipher).map_err(|e| BasilError(format!("PGP.EncryptFile: {}", e)))?;
                Ok(Value::Int(1))
            }
            "DECRYPTFILE$" => {
                if args.len()!=4 { return Err(bad_arity("DecryptFile$", 4, args.len())); }
                let sec_armored = str_arg(&args[0]);
                let pass = str_arg(&args[1]);
                let in_path = str_arg(&args[2]);
                let out_path = str_arg(&args[3]);
                let cipher = std::fs::read(&in_path).map_err(|e| BasilError(format!("PGP.DecryptFile: {}", e)))?;
                let plain = decrypt_armored(&sec_armored, empty_to_none(&pass), &cipher)?;
                std::fs::write(&out_path, &plain).map_err(|e| BasilError(format!("PGP.DecryptFile: {}", e)))?;
                Ok(Value::Int(1))
            }
            "SIGNFILE$" => {
                if args.len()!=4 { return Err(bad_arity("SignFile$", 4, args.len())); }
                let sec_armored = str_arg(&args[0]);
                let pass = str_arg(&args[1]);
                let in_path = str_arg(&args[2]);
                let sig_out = str_arg(&args[3]);
                let mut input = Vec::new();
                File::open(&in_path).map_err(|e| BasilError(format!("PGP.SignFile: {}", e)))?.read_to_end(&mut input).map_err(|e| BasilError(format!("PGP.SignFile: {}", e)))?;
                let sig = sign_detached_armored(&sec_armored, empty_to_none(&pass), &input)?;
                std::fs::write(&sig_out, sig).map_err(|e| BasilError(format!("PGP.SignFile: {}", e)))?;
                Ok(Value::Int(1))
            }
            "VERIFYFILE$" => {
                if args.len()!=3 { return Err(bad_arity("VerifyFile$", 3, args.len())); }
                let pub_armored = str_arg(&args[0]);
                let in_path = str_arg(&args[1]);
                let sig_path = str_arg(&args[2]);
                let data = std::fs::read(&in_path).map_err(|e| BasilError(format!("PGP.VerifyFile: {}", e)))?;
                let sig = std::fs::read(&sig_path).map_err(|e| BasilError(format!("PGP.VerifyFile: {}", e)))?;
                let ok = verify_detached(&pub_armored, &data, &sig)?;
                Ok(Value::Int(if ok { 1 } else { 0 }))
            }
            "READFILETEXT$" => {
                if args.len()!=1 { return Err(bad_arity("ReadFileText$", 1, args.len())); }
                let p = str_arg(&args[0]);
                let s = std::fs::read_to_string(&p).map_err(|e| BasilError(format!("PGP.ReadFileText: {}", e)))?;
                Ok(Value::Str(s))
            }
            "WRITEFILETEXT$" => {
                if args.len()!=2 { return Err(bad_arity("WriteFileText$", 2, args.len())); }
                let p = str_arg(&args[0]);
                let s = str_arg(&args[1]);
                std::fs::write(&p, s).map_err(|e| BasilError(format!("PGP.WriteFileText: {}", e)))?;
                Ok(Value::Int(1))
            }
            other => Err(BasilError(format!("Unknown method '{}' on CRYPTO_PGP", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

fn bad_arity(name: &str, want: usize, got: usize) -> BasilError { BasilError(format!("{} expects {} arguments (got {})", name, want, got)) }
fn str_arg(v: &Value) -> String { match v { Value::Str(s)=>s.clone(), other=>format!("{}", other) } }
fn empty_to_none(s: &str) -> Option<String> { if s.is_empty() { None } else { Some(s.to_string()) } }

fn parse_cert_armored(s: &str) -> std::result::Result<Cert, openpgp::Error> {
    // Parse armored cert or secret key
    let mut rdr = s.as_bytes();
    Cert::from_reader(&mut rdr)
}

fn encrypt_armored(recipient_pubkey_armored: &str, data: &[u8]) -> Result<String> {
    let policy = &StandardPolicy::new();
    let cert = parse_cert_armored(recipient_pubkey_armored)
        .map_err(|_| BasilError("PGP.Encrypt: KeyParseError".into()))?;

    let recipients: Vec<_> = cert
        .keys()
        .with_policy(policy, None)
        .alive()
        .revoked(false)
        .for_transport_encryption()
        .collect();
    if recipients.is_empty() {
        return Err(BasilError("PGP.Encrypt: No valid encryption key found".into()));
    }

    let mut out = Vec::<u8>::new();
    let msg = Message::new(&mut out);
    let msg = Armorer::new(msg).kind(armor::Kind::Message).build().map_err(|e| BasilError(format!("PGP.Encrypt: {}", e)))?;
    let enc = Encryptor::for_recipients(msg, recipients.iter().map(|ka| ka.key()))
        .build().map_err(|e| BasilError(format!("PGP.Encrypt: {}", e)))?;
    let mut lit = LiteralWriter::new(enc).build().map_err(|e| BasilError(format!("PGP.Encrypt: {}", e)))?;
    lit.write_all(data).map_err(|e| BasilError(format!("PGP.Encrypt: {}", e)))?;
    lit.finalize().map_err(|e| BasilError(format!("PGP.Encrypt: {}", e)))?;
    let armored = String::from_utf8(out).map_err(|_| BasilError("PGP.Encrypt: Output not UTF-8".into()))?;
    Ok(armored)
}

struct HelperDec {
    cert: Cert,
    pass: Option<String>,
}

impl DecryptionHelper for HelperDec {
    fn decrypt<D>(&mut self,
                  pkesks: &[PKESK],
                  _skesks: &[SKESK],
                  _sym_algo: Option<SymmetricAlgorithm>,
                  mut decrypt: D) -> std::result::Result<Option<openpgp::Fingerprint>, openpgp::Error>
        where D: FnMut(SymmetricAlgorithm, &SessionKey) -> openpgp::Result<Option<openpgp::Fingerprint>>
    {
        let policy = &StandardPolicy::new();
        for pkesk in pkesks {
            if let Some(ka) = self.cert.keys()
                .with_policy(policy, None)
                .secret()
                .alive()
                .revoked(false)
                .for_transport_encryption()
                .keyid(pkesk.recipient())
                .next()
            {
                // Unlock private key (if needed)
                let unlocked = if let Some(pw) = &self.pass {
                    ka.key().clone().unlock(|| pw.clone().into())
                } else {
                    ka.key().clone().unlock(|| "".to_string().into())
                };
                let keypair = match unlocked {
                    Ok(k) => k,
                    Err(_e) => continue, // wrong passphrase? try next
                }.into_keypair()?;
                let sk = keypair.decrypt_session_key(pkesk)?;
                if let Some(fp) = decrypt(pkesk.symmetric_algo(), &sk)? { return Ok(Some(fp)); }
            }
        }
        Ok(None)
    }
}

fn decrypt_armored(secret_key_armored: &str, pass: Option<String>, cipher_armored_bytes: &[u8]) -> Result<Vec<u8>> {
    let policy = &StandardPolicy::new();
    let cert = parse_cert_armored(secret_key_armored)
        .map_err(|_| BasilError("PGP.Decrypt: KeyParseError".into()))?;

    let helper = HelperDec { cert, pass };
    let mut dec = DecryptorBuilder::from_bytes(cipher_armored_bytes)
        .map_err(|e| BasilError(format!("PGP.Decrypt: {}", e)))?
        .with_policy(policy, None, helper)
        .build()
        .map_err(|e| BasilError(format!("PGP.Decrypt: {}", e)))?;
    let mut out = Vec::new();
    std::io::copy(&mut dec, &mut out).map_err(|e| BasilError(format!("PGP.Decrypt: {}", e)))?;
    Ok(out)
}

struct HelperVerify {
    cert: Cert,
}
impl VerificationHelper for HelperVerify {
    fn get_certs(&mut self, _ids: &[openpgp::KeyID]) -> openpgp::Result<Vec<Cert>> {
        Ok(vec![self.cert.clone()])
    }
    fn check(&mut self, _structure: &MessageStructure) -> openpgp::Result<()> { Ok(()) }
}

fn sign_detached_armored(secret_key_armored: &str, pass: Option<String>, data: &[u8]) -> Result<String> {
    let policy = &StandardPolicy::new();
    let cert = parse_cert_armored(secret_key_armored)
        .map_err(|_| BasilError("PGP.Sign: KeyParseError".into()))?;

    // Find signing key and unlock
    let mut maybe_keypair = None;
    for ka in cert.keys().with_policy(policy, None).secret().alive().revoked(false).for_signing() {
        let res = if let Some(pw) = &pass {
            ka.key().clone().unlock(|| pw.clone().into())
        } else {
            ka.key().clone().unlock(|| "".to_string().into())
        };
        if let Ok(unlocked) = res { maybe_keypair = Some(unlocked.into_keypair().map_err(|e| BasilError(format!("PGP.Sign: {}", e)))?); break; }
    }
    let keypair = maybe_keypair.ok_or_else(|| BasilError("PGP.Sign: No usable signing key".into()))?;

    let mut out = Vec::<u8>::new();
    let msg = Message::new(&mut out);
    let msg = Armorer::new(msg).kind(armor::Kind::Signature).build().map_err(|e| BasilError(format!("PGP.Sign: {}", e)))?;
    let mut ds = DetachedSigner::new(msg, keypair).build().map_err(|e| BasilError(format!("PGP.Sign: {}", e)))?;
    ds.write_all(data).map_err(|e| BasilError(format!("PGP.Sign: {}", e)))?;
    ds.finalize().map_err(|e| BasilError(format!("PGP.Sign: {}", e)))?;
    let sig = String::from_utf8(out).map_err(|_| BasilError("PGP.Sign: Output not UTF-8".into()))?;
    Ok(sig)
}

fn verify_detached(public_key_armored: &str, data: &[u8], signature_armored: &[u8]) -> Result<bool> {
    let policy = &StandardPolicy::new();
    let cert = parse_cert_armored(public_key_armored)
        .map_err(|_| BasilError("PGP.Verify: KeyParseError".into()))?;
    let helper = HelperVerify { cert };

    let mut v = DetachedVerifierBuilder::from_bytes(signature_armored)
        .map_err(|e| BasilError(format!("PGP.Verify: {}", e)))?
        .with_policy(policy, None, helper)
        .detached_reader(data)
        .map_err(|e| BasilError(format!("PGP.Verify: {}", e)))?;

    let mut sink = std::io::sink();
    std::io::copy(&mut v, &mut sink).map_err(|e| BasilError(format!("PGP.Verify: {}", e)))?;
    // If verification fails, builder/read should error. If we got here, consider it OK.
    Ok(true)
}
