# Junie Pro task: Basil **AWS Feature Objects – Phase 1** (S3, SES, SQS + AWS context)

## Context

Basil is a BASIC-flavored language with feature-gated objects (e.g., obj-term, obj-ai). We want a first pass of **AWS** integrations focused on the everyday developer needs, behind Cargo features, with a Basil-native surface (simple strings, ints, arrays, JSON strings), and errors mapped to Basil exceptions (usable with `TRY/CATCH/FINALLY`).

**Important process notes:**

* Do **not** perform git actions (no branches/PRs/commits).
* No formal tests required in this pass.
* **You must include runnable example programs under `/examples/` and an end-user doc.** (This is part of acceptance.)

## Scope (Phase 1 deliverables)

Implement these objects and methods:

### 0) Shared context (optional but recommended)

**`AWS@`** (context/helper)

* Purpose: hold SDK config (region, profile, timeouts, retry policy) and create service clients efficiently.
* Constructor:

  * `DIM aws@ AS AWS()` — auto-discovers creds (env, `~/.aws/credentials`, `AWS_PROFILE`, IMDS) and region (`AWS_REGION`/`AWS_DEFAULT_REGION`).
  * Optional properties: `Profile$`, `Region$`, `MaxRetries%`, `TimeoutMs%`.
* Methods:

  * `MakeS3()` → `AWS_S3`
  * `MakeSES()` → `AWS_SES`
  * `MakeSQS()` → `AWS_SQS`
  * (Optionally) `AssumeRole$(role_arn$, session_name$, duration_sec%?)` → updates context creds; returns JSON with temp keys.

> If this is too much for Phase 1, you may inline config in each service object constructor and add `AWS@` in Phase 2—but prefer `AWS@` now if feasible.

### 1) **S3** — `AWS_S3`

* Methods:

  * `Put$(bucket$, key$, data$ | file$)` → ETag$
  * `Get$(bucket$, key$)` → bytes$  (string of raw bytes; document that it might be binary)
  * `GetToFile(bucket$, key$, file$)` → ok%
  * `List$(bucket$, prefix$?, max%?)` → String[]  (object keys; handle pagination internally)
  * `Delete(bucket$, key$)` → ok%
  * `SignedUrl$(bucket$, key$, expires_sec%)` → url$

### 2) **SES** — `AWS_SES`

* Methods:

  * `SendEmail(to$, subject$, body$, from$?, reply_to$?, is_html%?)` → message_id$
  * `SendRaw$(mime$)` → message_id$

### 3) **SQS** — `AWS_SQS`

* Methods:

  * `Send$(queue_url$, body$, delay_sec%?)` → message_id$
  * `Receive$(queue_url$, max%?, wait_sec%?, vis_timeout_sec%?)` → String[]
    *(Return an array of JSON strings with at least: `MessageId`, `ReceiptHandle`, `Body`.)*
  * `Delete$(queue_url$, receipt_handle$)` → ok%
  * `Purge(queue_url$)` → ok%  *(guard: AWS 60-sec cooldown; surface errors cleanly)*

## API Shape & Conventions

* All methods return Basil primitives (`$`, `%`, arrays of `$`) or JSON strings for structured results.
* **Errors:** map SDK errors to Basil exceptions with clear messages (service, operation, code); e.g., `S3.Put: AccessDenied (check bucket/policy)`. These should be catchable with `TRY … CATCH err$`.
* **Credentials/Region:** honor AWS default provider chain (env, shared profile via `AWS_PROFILE`, IMDS) and `AWS_REGION`/`AWS_DEFAULT_REGION`.
* **Feature gating:** new crate `basil-objects-aws/` with Cargo features:

  * `obj-aws-s3`, `obj-aws-ses`, `obj-aws-sqs`, and umbrella `obj-aws = ["obj-aws-s3","obj-aws-ses","obj-aws-sqs"]`.
* **Workspace deps:** wire via `[workspace.dependencies]` (no backslash paths).

## Implementation Notes (Rust)

* Create crate `basil-objects-aws/` exporting Basil object factories to the existing object registry.
* Use the official AWS Rust SDK:

  * `aws-config` for shared config (profile/region/assume role if `AWS@` implemented).
  * `aws-sdk-s3`, `aws-sdk-sesv2`, `aws-sdk-sqs`.
* Each object implements the core `BasicObject` trait already used by other feature objects (get/set props, call methods, `descriptor()` for `DESCRIBE`).
* For `List$`/`Receive$`, handle pagination (next tokens) under the hood; return up to `max%` if supplied, else a sane default.
* Binary note: `Get$` returns bytes as a Basil string; recommend `GetToFile` for large/binary files in docs.

## Security & Reliability

* Never log secrets. Redact sensitive fields in error messages.
* Respect SDK retry/backoff defaults; expose minimal knobs via `AWS@` if present.
* Timeouts: use SDK config or per-call optional timeouts if trivial; otherwise document defaults.

## Docs (must include)

Create **`docs/guides/AWS.md`** that explains for end users:

* What Phase 1 supports (S3, SES, SQS) and the Basil object names/methods.
* How to configure credentials:

  * Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, optional `AWS_SESSION_TOKEN`),
  * `AWS_REGION` / `AWS_DEFAULT_REGION`,
  * Shared config (`~/.aws/credentials`, `~/.aws/config`) and `AWS_PROFILE`.
* Minimal IAM permissions per service (example JSON policies or links):

  * S3: `s3:PutObject`, `s3:GetObject`, `s3:DeleteObject`, `s3:ListBucket`.
  * SES: `ses:SendEmail`, `ses:SendRawEmail`.
  * SQS: `sqs:SendMessage`, `sqs:ReceiveMessage`, `sqs:DeleteMessage`, `sqs:PurgeQueue`, `sqs:ChangeMessageVisibility`.
* How errors surface in Basil and how to handle them with `TRY/CATCH`.
* Feature flags: how to build with `--features obj-aws` or specific services.
* Quickstart snippets (from examples below).

## Examples (must include; runnable)

Place these under `/examples/aws/`:

1. **`aws_quickstart.basil`** — minimal setup + sanity checks

```
REM Quickstart: verify AWS config & region
DIM aws@ AS AWS()
PRINT "Region: #{aws@.Region$}"
PRINT "Profile: #{aws@.Profile$}"
```

2. **`aws_s3_basic.basil`** — put/get/list/delete/signed URL

```
#USE AWS_S3
DIM aws@ AS AWS()
DIM s3@ = aws@.MakeS3()

DIM bucket$ = "your-bucket"
DIM key$ = "basil-demo/hello.txt"

TRY
  PRINT "Uploading..."
  PRINT "ETag: ", s3@.Put$(bucket$, key$, "Hello from Basil!")
  PRINT "Listing..."
  DIM keys$ = s3@.List$(bucket$, "basil-demo/", 100)
  FOR EACH k$ IN keys$ : PRINT " - ", k$ : NEXT
  PRINT "Signed URL: ", s3@.SignedUrl$(bucket$, key$, 300)
  PRINT "Download: ", s3@.GetToFile(bucket$, key$, "hello.txt")
  PRINT "Deleting: ", s3@.Delete(bucket$, key$)
CATCH err$
  PRINT "S3 error: ", err$
END TRY
```

3. **`aws_ses_send.basil`** — send email

```
#USE AWS_SES
DIM aws@ AS AWS()
DIM ses@ = aws@.MakeSES()

TRY
  DIM id$ = ses@.SendEmail("dev@example.com", "Hello from Basil", "<b>Hi!</b>", "noreply@example.com", "", 1)
  PRINT "SES MessageId: ", id$
CATCH e$
  PRINT "SES failed: ", e$
END TRY
```

4. **`aws_sqs_worker.basil`** — basic receive/delete loop

```
#USE AWS_SQS
DIM aws@ AS AWS()
DIM sqs@ = aws@.MakeSQS()

DIM q$ = "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue"

PRINT "Polling once..."
TRY
  DIM msgs$ = sqs@.Receive$(q$, 5, 10, 30)
  FOR EACH m$ IN msgs$
    PRINT "Msg: ", m$
    REM Extract ReceiptHandle from JSON (left to user or JSON helper)
  NEXT
CATCH e$
  PRINT "SQS error: ", e$
END TRY
```

*(If `AWS@` isn’t implemented in Phase 1, replace `aws@.MakeX()` with `DIM s3@ AS AWS_S3()` / `DIM ses@ AS AWS_SES()` / `DIM sqs@ AS AWS_SQS()` that internally discover config.)*

## Wiring & Features

* Add crate: `basil-objects-aws/` with modules `s3.rs`, `ses.rs`, `sqs.rs`, and optional `context.rs`.
* Export registration functions guarded by features:

  * `obj-aws-s3`, `obj-aws-ses`, `obj-aws-sqs`, `obj-aws`.
* Update the root workspace to include the crate and features using **forward slashes** and `[workspace.dependencies]`.

## Acceptance checklist (must meet)

* [ ] Workspace builds with `--features obj-aws` and also when features are off.
* [ ] `/examples/aws/quickstart.basil`, `/examples/aws/s3_basic.basil`, `/examples/aws/ses_send.basil`, `/examples/aws/sqs_worker.basil` exist and run (with valid creds).
* [ ] `docs/guides/AMAZON_AWS.md` exists and clearly explains setup, permissions, feature flags, usage patterns, and error handling with `TRY/CATCH`.
* [ ] `DESCRIBE s3@`/`DESCRIBE ses@`/`DESCRIBE sqs@` show methods and argument hints via the object descriptors.
* [ ] Errors from AWS map to Basil exceptions with readable messages (service/op/code).

## Follow-ups (not in this task; note in AMAZON_AWS.md)

* Phase 2 services: Lambda (Invoke), DynamoDB, Secrets Manager, SSM, CloudWatch Logs, STS AssumeRole (if not added now), and Polly (TTS).
* Optional: streaming/multipart helpers (S3 multipart for very large files), JSON helper utilities for easier receipt parsing.

---

If anything is ambiguous (e.g., whether to include `AWS@` now), default to implementing `AWS@`—it will make examples cleaner and reduce client re-initialization overhead.
