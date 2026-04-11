use serde::{Deserialize, Serialize};

/// Unique identifier for a symbol across the entire codebase.
/// Format: "relative/file/path.ts::SymbolName::kind"
/// Example: "src/models/user.ts::UserPayload::class"
/// Example: "src/models/user.ts::UserPayload::userId::property"
pub type SymbolId = String;

/// A symbol — any named entity in source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub language: Language,
    pub file: String,
    pub line_start: u32,
    pub line_end: u32,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Class,
    Interface,
    TypeAlias,
    Function,
    Method,
    Property,
    Variable,
    Module,
    Enum,
    EnumVariant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    JavaScript,
    Python,
    Rust,
    Go,
    Java,
}

/// A directed relationship between two symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: SymbolId,
    pub to: SymbolId,
    pub kind: RelationshipKind,
    pub alias: Option<String>,
    pub properties_accessed: Vec<String>,
    pub context: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipKind {
    Imports,
    Calls,
    Extends,
    Implements,
    UsesType,
    AccessesProperty,
    ReExports,
    Instantiates,
}

/// The complete IR output from one file parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIR {
    pub file: String,
    pub language: Language,
    pub symbols: Vec<Symbol>,
    pub relationships: Vec<Relationship>,
    pub parse_errors: Vec<String>,
}

/// The complete IR output from a full repo or incremental update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoIR {
    pub root: String,
    pub files: Vec<FileIR>,
    pub language_stats: std::collections::HashMap<String, usize>,
}
