Love this—Basil is a great fit for “everyday AWS,” as long as we keep the surface tight and opt-in. Here’s a practical cut of **~10 services** that cover 90% of what most app devs need, plus how I’d shape them as Basil Feature Objects.

# The “everyday AWS 10”

1. **S3** — upload/download, list prefixes (“folders”), delete, signed URLs.
2. **SES** — send simple/HTML email, attachments (raw), handle sandbox quirks.
3. **SQS** — push/pull/delete messages, visibility timeout, batch ops.
4. **SNS** — publish messages (topic ARN), simple SMS/email publish.
5. **Lambda (Invoke)** — sync invoke with JSON payload, get logs if requested.
6. **DynamoDB** — put/get, query by key + begins_with, simple update, scan (guarded).
7. **Secrets Manager** — get/put secrets by name/ARN; JSON helpers.
8. **SSM Parameter Store** — get/put parameters (SecureString support).
9. **CloudWatch Logs** — write log events, tail recent events for a log group/stream.
10. **STS (AssumeRole)** — temporary creds; supports cross-account; feeds other objects.

Nice-to-haves later: **Polly** (TTS synth to file), **EventBridge** (put events), **Textract** (OCR), **Rekognition** (image labels). I’d keep **IAM**, **Organizations**, **Ground Station**, etc., *out* of scope.

# How users configure it (simple & familiar)

* Honor standard AWS envs and config files automatically:

    * `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN` (optional),
    * `AWS_REGION` / `AWS_DEFAULT_REGION`,
    * shared config/credentials from `~/.aws/credentials` & `~/.aws/config` with `AWS_PROFILE`.
* Optional Basil config overlay: `./.basil-aws.toml` with `{profile, region, assume_role_arn, session_name, endpoint_override}`.
* Provide one **bootstrap object** that captures context:

    * `DIM aws@ AS AWS(profile$?, region$?)`
    * Or rely entirely on env/profile discovery and let per-service objects accept overrides.

# Object model (Basil-native, opt-in)

Each service gets a feature-gated object in `obj-aws-*` (using the official AWS Rust SDK under the hood):

* **AWS_S3**

    * `Put$(bucket$, key$, data$ | file$)` → ETag$
    * `Get$(bucket$, key$)` → bytes$
    * `GetToFile(bucket$, key$, file$)` → ok%
    * `List$(bucket$, prefix$?, max%?)` → String[] (keys)
    * `Delete(bucket$, key$)` → ok%
    * `SignedUrl$(bucket$, key$, expires_sec%)` → url$

* **AWS_SES**

    * `SendEmail(to$, subject$, html$ | text$, from$?, reply_to$?)` → message_id$
    * `SendRaw$(mime$)` → message_id$

* **AWS_SQS**

    * `Send$(queue_url$, body$, delay_sec%?)` → message_id$
    * `Receive$(queue_url$, max%?, wait_sec%?, vis_timeout_sec%?)` → String[] (receipt-packed JSONs)
    * `Delete$(queue_url$, receipt_handle$)` → ok%
    * (Optionally) `Purge(queue_url$)` → ok%

* **AWS_SNS**

    * `Publish$(topic_arn$, message$, subject$?)` → message_id$

* **AWS_LAMBDA**

    * `Invoke$(function$, payload_json$?, log_tail%?)` → response_json$ (and maybe set `LastLog$`)

* **AWS_DDB**

    * `Put$(table$, item_json$)` → ok%
    * `Get$(table$, key_json$)` → item_json$
    * `Query$(table$, key_cond_expr$, expr_vals_json$, index$?)` → items_json$
    * `Update$(table$, key_json$, update_expr$, expr_vals_json$)` → item_json$
    * `Scan$(table$, filter_expr$?, expr_vals_json$?)` → items_json$ (warn in docs)

* **AWS_SECRETS**

    * `Get$(name$)` → secret$
    * `Put$(name$, secret$, kms_key_id$?)` → version_id$

* **AWS_SSM**

    * `GetParam$(name$, with_decrypt%?)` → value$
    * `PutParam$(name$, value$, secure%?)` → version%

* **AWS_CW_LOGS**

    * `Put$(group$, stream$, message$)` → ok% (create-if-not-exists)
    * `Tail$(group$, stream$, since_sec%?)` → String[] (messages)

* **AWS_STS**

    * `AssumeRole$(role_arn$, session_name$, duration_sec%?)` → creds_json$ (also let `AWS@` re-hydrate context)

> All methods return simple Basil types (`$`, `%`, arrays of `$`) and JSON strings for structured data (DynamoDB items, Lambda payloads). That keeps the Basil surface small and predictable.

# How it compiles & links (clean and small)

* **Crate: `basil-objects-aws/`** with per-service modules gated by Cargo features:
  `obj-aws-s3`, `obj-aws-ses`, `obj-aws-sqs`, … and umbrella `obj-aws-common`, `obj-aws-all`.
* Use the **AWS Rust SDK** crates (they’re GA) *feature-gated per service* to avoid pulling half of AWS.
* Optional tiny **`AWS@` context object** that owns a shared SDK config/client cache and exposes `MakeS3()`, `MakeSQS()`, etc., to reduce per-call re-init cost.

# Error handling & reliability (important)

* Every method maps SDK errors → Basil exceptions, so users can:

  ```basic
  TRY
    s3@.Put$("my-bucket", "a.txt", "hello")
  CATCH err$
    PRINT "S3 failed: ", err$
  END TRY
  ```
* Built-in **retries** with exponential backoff (respect SDK defaults; expose a couple of tunables via AWS@ properties: `MaxRetries%`, `TimeoutMs%`).
* **Cancellation & timeouts**: per-call timeout arg (optional) or global default in `AWS@`.
* **Costs & throttling**: note in docs where operations are chatty (DDB `Scan`, `Tail` loops, S3 list pagination).

# Security & credentials

* Default to AWS SDK’s **credential chain** (env → profile → EC2/IMDS → assume-role).
* Provide `AssumeRole$()` in STS to switch identities at runtime (updates AWS@), e.g.:

  ```basic
  aws@.AssumeRole$("arn:aws:iam::123:role/Writer", "basil-session", 3600)
  ```
* **No plaintext secret logging**. Redact in errors; show ARNs/regions only.

# Developer ergonomics in Basil

* Keep **JSON-in/out** for structured services; add `JSON.PARSE$()` / `JSON.STRINGIFY$()` helpers (if not already present) to let users pick fields easily.
* Provide **examples** in `/examples/aws_*`:

    * `aws_s3_basic.basil` (put/get/list/signed-url)
    * `aws_ses_send.basil`
    * `aws_sqs_worker.basil` (receive loop with visibility + delete)
    * `aws_lambda_invoke.basil`
    * `aws_ddb_crud.basil`
    * `aws_secrets_and_ssm.basil`
* A single **quickstart** doc: setting env/profile, region, and a minimal S3 upload.

# What to skip (for now)

* Admin planes: **IAM**, **Organizations**, **CloudFormation**, **EKS**, **RDS** (you already plan a SQL connector), **Ground Station**.
* Long-running streaming (Kinesis) until we decide on async/iterators for network streams.
* Complex SES/SMTP interactions (stick to API send).
* Multipart S3 upload beyond a single file helper—unless files > 5GB are a common ask.

# Why this will feel great in Basil

* **Small surface, big leverage**: each service exposes 4–8 “do the thing” methods; no need to learn the entire SDK.
* **Basil-native ergonomics**: simple scalars/arrays/JSON strings; exceptions integrate with your new `TRY/CATCH/FINALLY`.
* **Tunable footprint**: enable only what you ship (`--features obj-aws-s3,obj-aws-ses,...`).

---

If you like this direction, I can draft a ready-to-paste **Junie Pro prompt** that scaffolds `basil-objects-aws/`, wires the feature flags, and implements **S3 + SES + SQS** first (most demanded trio) with examples and docs—then we add Lambda/DDB/Secrets/SSM/CloudWatch/STS in phase 2.
