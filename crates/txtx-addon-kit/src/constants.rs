use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumString, IntoStaticStr};

/// Keys related to signer operations and signatures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum SignerKey {
    SignedMessageBytes,
    SignedTransactionBytes,
    TxHash,
    SignatureApproved,
    SignatureSkippable,
    ProvidePublicKeyActionResult,
    IsBalanceChecked,
}

/// Keys related to action items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActionItemKey {
    CheckAddress,
    CheckedAddress,
    CheckBalance,
    IsBalanceChecked,
    CheckNonce,
    CheckFee,
    CheckOutput,
    ProvidePublicKey,
    ProvideSignedTransaction,
    ProvideSignedSquadTransaction,
    SendTransaction,
    ReviewDeployedContract,
    Env,
    Genesis,
    ValidateBlock,
    BeginFlow,
    ReExecuteCommand,
    Diagnostic,
    Output,
    ProvideInput,
    CheckInput,
}

/// Keys related to nested constructs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum NestedConstructKey {
    NestedConstructDid,
    NestedConstructIndex,
    NestedConstructCount,
}

/// Keys related to documentation and metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum DocumentationKey {
    Description,
    MetaDescription,
    Markdown,
    MarkdownFilepath,
}

/// Keys related to runbook execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum RunbookKey {
    ThirdPartySignatureStatus,
    RunbookCompleteAdditionalInfo,
}
