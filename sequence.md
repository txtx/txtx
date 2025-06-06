
Starting a runbook through Verifying Addresses
```mermaid
sequenceDiagram
  actor User as User
  participant lib.rs as lib.rs
  participant eval as eval
  participant Squad Signer as Squad Signer
  participant Initiator Signer as Initiator Signer
  participant Squad Signer as Squad Signer
  participant send_token as send_token
  participant sign_transaction as sign_transaction

User ->> lib.rs: Start Runbook
lib.rs ->> eval: Run Signers Evaluation
eval ->> Squad Signer: Check Activability
Squad Signer --> User: Check Address
eval ->> Initiator Signer: Check Activability
Initiator Signer --> User: Check Address
User ->> lib.rs: Verify Address
```

Verified address through Activated signers:
```
sequenceDiagram
  actor User as User
  participant lib.rs as lib.rs
  participant eval as eval
  participant Squad Signer as Squad Signer
  participant Initiator Signer as Initiator Signer
  participant Squad Signer as Squad Signer
  participant send_token as send_token
  participant sign_transaction as sign_transaction
      
lib.rs ->> eval: Run Signers Evaluation
eval ->> Squad Signer: Activate
Squad Signer ->> Squad Signer: Compute Keys
Squad Signer ->> Initiator Signer: Activate
Initiator Signer ->> Squad Signer: Return Keys
Squad Signer ->> eval: Return Keys
eval ->> lib.rs: Signers Activated
```

Activated signers through approved proposal tx
```mermaid
sequenceDiagram
  actor User as User
  participant lib.rs as lib.rs
  participant eval as eval
  participant Squad Signer as Squad Signer
  participant Initiator Signer as Initiator Signer
  participant Squad Signer as Squad Signer
  participant send_token as send_token
  participant sign_transaction as sign_transaction


lib.rs ->> eval: Run Constructs Evaluation
eval ->> send_token: Check Signed Executability
send_token ->> send_token: Create Transaction
send_token ->> sign_transaction: Check Signed Executability
sign_transaction ->> Squad Signer: Check Signability
Squad Signer ->> Squad Signer: Get Initiator Signer
Squad Signer ->> sign_transaction: Check Signed Executability
sign_transaction ->> Initiator Signer: Check Signability
Initiator Signer->> User: Approve Tx Action
User->>lib.rs: Transaction Approved
```

Transaction approved through signed proposal
```mermaid
lib.rs ->> eval: Run Constructs Evaluation
eval ->> send_token: Check Signed Executability
send_token ->> Squad Signer: Check Signability
Squad Signer ->> send_token: No Action
send_token ->> eval: No Action
eval ->> send_token: Run Signed Execution
send_token ->> sign_transaction: Run Signed Execution
sign_transaction ->> Squad Signer: Sign
Squad Signer ->> Squad Signer: Wrap Tx in Proposal

