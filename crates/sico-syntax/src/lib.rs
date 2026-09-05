//! Lossless rowan syntax tree kinds and typed aliases.

#![forbid(unsafe_code)]

pub use rowan::{GreenNode, GreenNodeBuilder, NodeOrToken};
use rowan::{Language, SyntaxKind as RawSyntaxKind};

/// Raw syntax kind. Node kinds occupy low values; lexer token kinds are encoded
/// from [`TOKEN_BASE`] without coupling this crate back to the lexer crate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SyntaxKind(u16);

impl SyntaxKind {
    pub const ROOT: Self = Self(0);
    pub const NEWTYPE_DECL: Self = Self(1);
    pub const RECORD_DECL: Self = Self(2);
    pub const ENUM_DECL: Self = Self(3);
    pub const CAPABILITY_DECL: Self = Self(4);
    pub const RESOURCE_DECL: Self = Self(5);
    pub const INTERFACE_DECL: Self = Self(6);
    pub const FUNCTION_DECL: Self = Self(7);
    pub const MATCH_BLOCK: Self = Self(8);
    pub const IF_BLOCK: Self = Self(9);
    pub const USING_BLOCK: Self = Self(10);
    pub const TASK_BLOCK: Self = Self(11);
    pub const ERROR: Self = Self(12);
    pub const MISSING: Self = Self(13);
    pub const WHILE_BLOCK: Self = Self(14);
    pub const TOKEN_BASE: u16 = 1_000;

    /// Encodes a lexer token discriminant as a rowan syntax kind.
    #[must_use]
    pub const fn token(raw_token_kind: u16) -> Self {
        Self(Self::TOKEN_BASE + raw_token_kind)
    }

    /// Whether this kind represents a lexer token.
    #[must_use]
    pub const fn is_token(self) -> bool {
        self.0 >= Self::TOKEN_BASE
    }

    /// Stable raw representation for snapshots and adapters.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// Rowan language marker for Sico.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SicoLanguage {}

impl Language for SicoLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
        SyntaxKind(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
        RawSyntaxKind(kind.0)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<SicoLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<SicoLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<SicoLanguage>;

/// Creates the typed root node for one completed green tree.
#[must_use]
pub fn root(green: GreenNode) -> SyntaxNode {
    SyntaxNode::new_root(green)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_kind_roundtrip_preserves_node_and_token_domains() {
        let node = SyntaxKind::FUNCTION_DECL;
        let token = SyntaxKind::token(42);
        assert_eq!(
            SicoLanguage::kind_from_raw(SicoLanguage::kind_to_raw(node)),
            node
        );
        assert_eq!(
            SicoLanguage::kind_from_raw(SicoLanguage::kind_to_raw(token)),
            token
        );
        assert!(!node.is_token());
        assert!(token.is_token());
    }
}
