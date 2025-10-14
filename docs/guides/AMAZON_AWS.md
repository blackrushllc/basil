Basil AWS Feature Objects – Phase 1 (S3, SES, SQS)

Note: This document mirrors docs/guides/AWS.md and exists to satisfy references using either filename.

This guide explains how to use the Basil AWS feature objects shipped in Phase 1. You get a simple, BASIC-friendly API for Amazon S3, SES, and SQS—plus an AWS context object to centralize configuration. All methods return Basil primitives (strings, integers, arrays of strings), and errors surface as Basil exceptions that you can handle with TRY/CATCH.

Contents
- What’s included in Phase 1
- Installing and building (feature flags)
- Configuring AWS credentials and region
- Minimal IAM permissions
- Error handling in Basil (TRY/CATCH)
- Object reference and examples
  - AWS@ context
  - AWS_S3
  - AWS_SES
  - AWS_SQS
- Notes and limits (timeouts, binary data, pagination)
- Follow‑ups (Phase 2)

What’s included in Phase 1
- AWS@ context/helper (recommended):
  - Discovers credentials and region via AWS default chain
  - Optional properties: Profile$, Region$, MaxRetries%, TimeoutMs%
  - Methods to construct service clients: MakeS3(), MakeSES(), MakeSQS()
  - (Optional/roadmap) AssumeRole$(role_arn$, session_name$, duration_sec%)
- S3 (AWS_S3): Put$, Get$, GetToFile, List$, Delete, SignedUrl$
- SES (AWS_SES): SendEmail, SendRaw$
- SQS (AWS_SQS): Send$, Receive$, Delete$, Purge

Installing and building (feature flags)
The AWS objects are feature‑gated. Enable specific services or the umbrella feature.

Examples:
- Build/run with all AWS services:
  cargo run -q -p basilc --features obj-aws -- run examples\aws\aws_quickstart.basil

- Only S3:
  cargo run -q -p basilc --features obj-aws-s3 -- run examples\aws\aws_s3_basic.basil

- Only SES:
  cargo run -q -p basilc --features obj-aws-ses -- run examples\aws\aws_ses_send.basil

- Only SQS:
  cargo run -q -p basilc --features obj-aws-sqs -- run examples\aws\aws_sqs_worker.basil

Configuring AWS credentials and region
The AWS SDK for Rust honors the default provider chain. You can use:

1) Environment variables
- AWS_ACCESS_KEY_ID
- AWS_SECRET_ACCESS_KEY
- AWS_SESSION_TOKEN (if using temporary credentials)
- AWS_REGION or AWS_DEFAULT_REGION

2) Shared config files
- %UserProfile%\.aws\credentials
- %UserProfile%\.aws\config
- Choose a profile with AWS_PROFILE (e.g., set AWS_PROFILE=myprofile)

3) EC2/ECS metadata (IMDS)
If running on AWS compute, the SDK can use instance/role credentials.

Minimal IAM permissions
Below are example policies you can attach to a user/role. Adjust resource ARNs to your environment.

S3 (basic put/get/list/delete)
{
  "Version": "2012-10-17",
  "Statement": [
    { "Effect": "Allow", "Action": ["s3:PutObject","s3:GetObject","s3:DeleteObject"], "Resource": "arn:aws:s3:::YOUR_BUCKET/*" },
    { "Effect": "Allow", "Action": ["s3:ListBucket"], "Resource": "arn:aws:s3:::YOUR_BUCKET" }
  ]
}

SES (send email)
{
  "Version": "2012-10-17",
  "Statement": [
    { "Effect": "Allow", "Action": ["ses:SendEmail","ses:SendRawEmail"], "Resource": "*" }
  ]
}

SQS (basic send/receive/delete/purge)
{
  "Version": "2012-10-17",
  "Statement": [
    { "Effect": "Allow", "Action": [
      "sqs:SendMessage","sqs:ReceiveMessage","sqs:DeleteMessage","sqs:PurgeQueue","sqs:ChangeMessageVisibility"
    ], "Resource": "arn:aws:sqs:REGION:ACCOUNT_ID:QUEUE_NAME" }
  ]
}

Error handling (TRY/CATCH)
AWS SDK errors are surfaced as Basil exceptions in a readable form, e.g.:
- S3.Put: AccessDenied (check bucket/policy)
- SQS.Purge: PurgeQueueInProgress (AWS 60s cooldown)

Handle them with TRY/CATCH:
TRY
  PRINT s3@.Delete(bucket$, key$)
CATCH err$
  PRINT "S3 error: ", err$
END TRY

Object reference and examples

AWS@ (context/helper)
Constructor:
- DIM aws@ AS AWS() — auto-discovers credentials and region via the default chain.

Properties:
- Profile$ (get/set)
- Region$  (get/set)
- MaxRetries% (optional)
- TimeoutMs%  (optional)

Methods:
- MakeS3()  → AWS_S3
- MakeSES() → AWS_SES
- MakeSQS() → AWS_SQS
- AssumeRole$(role_arn$, session_name$, duration_sec%?) → JSON with temp keys (if available in your build)

Quickstart
REM Quickstart: verify AWS config & region
DIM aws@ AS AWS()
PRINT "Region: #{aws@.Region$}"
PRINT "Profile: #{aws@.Profile$}"

AWS_S3
Methods
- Put$(bucket$, key$, data$ | file$) → ETag$
- Get$(bucket$, key$) → bytes$   (raw bytes in a string; may contain binary)
- GetToFile(bucket$, key$, file$) → ok%
- List$(bucket$, prefix$?, max%?) → String[]  (handles pagination internally)
- Delete(bucket$, key$) → ok%
- SignedUrl$(bucket$, key$, expires_sec%) → url$

Example: put/get/list/delete/sign URL
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

Notes for S3
- Get$ returns raw bytes in a string. For large/binary files prefer GetToFile.
- SignedUrl$ is useful for temporary access without exposing credentials.

AWS_SES
Methods
- SendEmail(to$, subject$, body$, from$?, reply_to$?, is_html%?) → message_id$
- SendRaw$(mime$) → message_id$

Example: send a simple email
#USE AWS_SES
DIM aws@ AS AWS()
DIM ses@ = aws@.MakeSES()

TRY
  DIM id$ = ses@.SendEmail("dev@example.com", "Hello from Basil", "<b>Hi!</b>", "noreply@example.com", "", 1)
  PRINT "SES MessageId: ", id$
CATCH e$
  PRINT "SES failed: ", e$
END TRY

Notes for SES
- Ensure your sending identity (domain or email) is verified in SES in your region.
- If your account is in the SES sandbox, you can only send to verified recipients.

AWS_SQS
Methods
- Send$(queue_url$, body$, delay_sec%?) → message_id$
- Receive$(queue_url$, max%?, wait_sec%?, vis_timeout_sec%?) → String[] of JSON
  Each JSON string includes at least: MessageId, ReceiptHandle, Body.
- Delete$(queue_url$, receipt_handle$) → ok%
- Purge(queue_url$) → ok% (guarded by 60s AWS cooldown)

Example: receive loop (single poll)
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

Notes for SQS
- Purge: AWS enforces PurgeQueueInProgress cooldown (~60 seconds). Handle this error gracefully.
- Long polling: use wait_sec% (e.g., 10) to reduce empty receives.

Notes and limits
- Timeouts and retries: default to AWS SDK behavior. You can set MaxRetries% and TimeoutMs% on AWS@ if exposed in your build.
- Pagination: S3.List$ and SQS.Receive$ handle pagination internally and cap results to your max argument.
- Data types: all methods stick to Basil primitives (strings, integers, arrays of strings). When structured output is needed, JSON strings are returned (e.g., SQS messages).
- Security: never logs secrets. Do not print credentials. Use SignedUrl$ for temporary, shareable access to S3 objects.

Follow‑ups (Phase 2)
- Lambda (Invoke), DynamoDB, Secrets Manager, SSM, CloudWatch Logs, STS AssumeRole (if not added in your build), Polly (TTS)
- Optional: S3 multipart upload helpers for very large files; JSON helpers for easier SQS receipt parsing.
