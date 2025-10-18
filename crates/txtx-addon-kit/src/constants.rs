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
}

/// Keys related to action items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ActionItemKey {
    #[strum(serialize = "check_address")]
    CheckAddress,
    #[strum(serialize = "checked_address")]
    CheckedAddress,
    #[strum(serialize = "check_balance")]
    CheckBalance,
    #[strum(serialize = "is_balance_checked")]
    IsBalanceChecked,
    #[strum(serialize = "begin_flow")]
    BeginFlow,
    #[strum(serialize = "re_execute_command")]
    ReExecuteCommand,
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
    DependsOn,
    MetaDescription,
    Markdown,
    MarkdownFilepath,
}

/// Keys related to conditions (pre/post)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ConditionKey {
    PreCondition,
    PostCondition,
}

/// Keys related to runbook execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum RunbookKey {
    ThirdPartySignatureStatus,
    RunbookCompleteAdditionalInfo,
}