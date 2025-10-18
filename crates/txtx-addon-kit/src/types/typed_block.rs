//! Typed wrappers around HCL blocks that parse construct types at creation time.
//!
//! This module provides two variants of typed block wrappers:
//!
//! ## `TypedBlock<'a>` - Borrowing Variant
//!
//! **Use when**: You already have a reference to a block and don't need to own it.
//!
//! **Benefits**:
//! - Zero-cost abstraction - no clones performed
//! - Ideal for visitor patterns and read-only traversals
//! - Works with existing references
//!
//! **Common use cases**:
//! ```ignore
//! // Visitor pattern - already have &Block
//! impl Visit for MyVisitor {
//!     fn visit_block(&mut self, block: &Block) {
//!         let typed_block = TypedBlock::new(block);  // No clone!
//!         match &typed_block.construct_type {
//!             Ok(ConstructType::Action) => { /* handle */ }
//!             _ => {}
//!         }
//!     }
//! }
//!
//! // Temporary analysis of a block
//! fn analyze_block(block: &Block) -> bool {
//!     let typed_block = TypedBlock::new(block);
//!     typed_block.construct_type.is_ok()
//! }
//! ```
//!
//! ## `OwnedTypedBlock` - Owning Variant
//!
//! **Use when**: You need to store blocks in collections or move them around.
//!
//! **Benefits**:
//! - Can be stored in collections like `VecDeque`, `Vec`, etc.
//! - Can be moved between functions
//! - Survives beyond the lifetime of the original block
//!
//! **Common use cases**:
//! ```ignore
//! // Parsing files into collections
//! let blocks: VecDeque<OwnedTypedBlock> =
//!     raw_content.into_typed_blocks()?;
//!
//! // Consuming blocks via iteration
//! while let Some(typed_block) = blocks.pop_front() {
//!     match &typed_block.construct_type {
//!         Ok(ConstructType::Import) => { /* process */ }
//!         _ => {}
//!     }
//! }
//!
//! // Storing blocks for later processing
//! let mut pending_blocks: Vec<OwnedTypedBlock> = Vec::new();
//! pending_blocks.push(OwnedTypedBlock::new(block));
//! ```
//!
//! ## Converting Between Variants
//!
//! ```ignore
//! // Borrowing variant can clone the inner block if needed
//! let typed_block = TypedBlock::new(&block);
//! let owned_block = typed_block.clone_inner();
//!
//! // Owned variant can be borrowed via Deref
//! let owned_typed_block = OwnedTypedBlock::new(block);
//! let span = owned_typed_block.span();  // Deref to &Block
//!
//! // Or extract the inner block
//! let block = owned_typed_block.into_inner();  // Consumes self, no clone
//! ```
//!
//! ## Design Philosophy
//!
//! The dual-variant design follows Rust's ownership patterns:
//! - Use `TypedBlock<'a>` like `&T` - when you have a reference
//! - Use `OwnedTypedBlock` like `T` - when you need ownership
//!
//! This matches the `str`/`String`, `Path`/`PathBuf` pattern in the standard library.
//!
//! ## Quick Decision Guide
//!
//! | Scenario | Use This | Reason |
//! |----------|----------|--------|
//! | Visitor pattern callback | `TypedBlock<'a>` | You receive `&Block` |
//! | Temporary read-only analysis | `TypedBlock<'a>` | No ownership needed |
//! | Storing in `Vec`/`VecDeque` | `OwnedTypedBlock` | Needs to outlive source |
//! | Parsing file into collection | `OwnedTypedBlock` | Use `into_typed_blocks()` |
//! | Moving between functions | `OwnedTypedBlock` | Ownership transfer |
//! | Iterating and consuming | `OwnedTypedBlock` | Use `pop_front()`, etc. |

use crate::hcl::structure::{Block, BlockLabel};
use crate::types::construct_type::ConstructType;
use std::str::FromStr;

/// A wrapper around `hcl_edit::Block` that parses the construct type at creation time.
///
/// This provides type-safe access to the block's construct type (action, variable, etc.)
/// instead of repeatedly parsing strings throughout the codebase.
///
/// # Ownership Model
///
/// **This is the borrowing variant** - it holds a reference to a block with lifetime `'a`.
/// Use this when you already have a `&Block` and don't need to own it.
///
/// For the owned variant that can be stored in collections, see [`OwnedTypedBlock`].
///
/// See the [module-level documentation](self) for a complete guide on when to use
/// each variant.
///
/// # Performance
///
/// - **Zero-cost abstraction** - no clones performed
/// - Parse construct type once at creation
/// - Deref coercion allows transparent access to `Block` methods
///
/// # Examples
///
/// ```ignore
/// // In a visitor pattern
/// impl Visit for MyVisitor {
///     fn visit_block(&mut self, block: &Block) {
///         let typed_block = TypedBlock::new(block);  // No clone!
///         match &typed_block.construct_type {
///             Ok(ConstructType::Action) => { /* handle action */ }
///             Ok(ConstructType::Variable) => { /* handle variable */ }
///             Err(unknown) => { /* handle unknown */ }
///         }
///     }
/// }
/// ```
///
/// # Deref Behavior
///
/// `TypedBlock` implements `Deref<Target = Block>`, allowing transparent access
/// to the underlying block's methods:
///
/// ```ignore
/// let typed_block = TypedBlock::new(&block);
/// let span = typed_block.span();  // Calls Block::span() via Deref
/// ```
#[derive(Debug, Clone)]
pub struct TypedBlock<'a> {
    /// The parsed construct type, or the original string if parsing failed
    pub construct_type: Result<ConstructType, String>,
    /// The underlying HCL block (borrowed)
    block: &'a Block,
}

impl<'a> TypedBlock<'a> {
    /// Creates a new `TypedBlock` by parsing the block's identifier.
    ///
    /// The construct type is determined once at creation time.
    /// The block is borrowed, not cloned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let typed_block = TypedBlock::new(&block);
    /// assert!(typed_block.construct_type.is_ok());
    /// ```
    pub fn new(block: &'a Block) -> Self {
        let construct_type = ConstructType::from_str(block.ident.value().as_str())
            .map_err(|_| block.ident.value().as_str().to_string());

        Self {
            construct_type,
            block,
        }
    }

    /// Returns the construct type identifier as a string.
    ///
    /// For successfully parsed construct types (Action, Variable, etc.),
    /// returns the canonical lowercase representation.
    /// For unknown or invalid construct types, returns the original
    /// identifier string that failed to parse.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let typed_block = TypedBlock::new(&block);
    /// assert_eq!(typed_block.ident_str(), "action");
    /// ```
    pub fn ident_str(&self) -> &str {
        match &self.construct_type {
            Ok(ct) => ct.as_ref(),
            Err(s) => s.as_str(),
        }
    }

    /// Clones the inner block.
    ///
    /// This is the only method that performs a clone, used when you need
    /// an owned copy of the block (e.g., for storage).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let typed_block = TypedBlock::new(&block);
    /// let owned_block = typed_block.clone_inner();
    /// ```
    pub fn clone_inner(&self) -> Block {
        self.block.clone()
    }

    /// Returns a reference to the inner block.
    ///
    /// This is a zero-cost operation that returns the borrowed block.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let typed_block = TypedBlock::new(&block);
    /// let block_ref = typed_block.inner();
    /// ```
    pub fn inner(&self) -> &Block {
        self.block
    }

    /// Extracts the first label as a string reference.
    ///
    /// Returns `None` if there are no labels or the first label is not a string.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let typed_block = TypedBlock::new(&block);
    /// if let Some(name) = typed_block.first_label_string() {
    ///     println!("First label: {}", name);
    /// }
    /// ```
    pub fn first_label_string(&self) -> Option<&str> {
        self.label_string(0)
    }

    /// Extracts a label at the given index as a string reference.
    ///
    /// Returns `None` if the index is out of bounds or the label is not a string.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let typed_block = TypedBlock::new(&block);
    /// if let (Some(first), Some(second)) =
    ///     (typed_block.label_string(0), typed_block.label_string(1)) {
    ///     println!("Labels: {}, {}", first, second);
    /// }
    /// ```
    pub fn label_string(&self, index: usize) -> Option<&str> {
        self.block.labels.get(index).and_then(|l| match l {
            BlockLabel::String(s) => Some(s.value().as_str()),
            _ => None,
        })
    }

    /// Extracts all string labels as a Vec.
    ///
    /// Non-string labels are filtered out.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let typed_block = TypedBlock::new(&block);
    /// let labels = typed_block.string_labels();
    /// ```
    pub fn string_labels(&self) -> Vec<&str> {
        self.block
            .labels
            .iter()
            .filter_map(|l| match l {
                BlockLabel::String(s) => Some(s.value().as_str()),
                _ => None,
            })
            .collect()
    }
}

/// Allow transparent access to the inner `Block`'s methods.
impl<'a> std::ops::Deref for TypedBlock<'a> {
    type Target = Block;

    fn deref(&self) -> &Self::Target {
        self.block
    }
}

/// An owned variant of `TypedBlock` that owns the block instead of borrowing it.
///
/// This is used when blocks need to be moved and stored, such as in collections
/// that are consumed via iteration (e.g., `VecDeque::pop_front()`).
///
/// # Ownership Model
///
/// **This is the owning variant** - it owns the `Block` internally.
/// Use this when you need to store blocks in collections, move them between functions,
/// or when the block needs to outlive its original source.
///
/// For the borrowing variant optimized for temporary usage, see [`TypedBlock`].
///
/// See the [module-level documentation](self) for a complete guide on when to use
/// each variant.
///
/// # Performance
///
/// - Owns the block (one allocation)
/// - Parse construct type once at creation
/// - Can be moved and stored without lifetime constraints
/// - Use `into_inner()` for zero-cost extraction of the block
///
/// # Examples
///
/// ```ignore
/// // Parsing file contents into a collection
/// let blocks: VecDeque<OwnedTypedBlock> =
///     raw_content.into_typed_blocks()?;
///
/// // Consuming blocks via iteration
/// while let Some(typed_block) = blocks.pop_front() {
///     match &typed_block.construct_type {
///         Ok(ConstructType::Import) => {
///             // Process import, potentially storing the block
///             pending_imports.push(typed_block);
///         }
///         Ok(ConstructType::Variable) => { /* handle */ }
///         _ => {}
///     }
/// }
/// ```
///
/// # Deref Behavior
///
/// `OwnedTypedBlock` implements `Deref<Target = Block>`, allowing transparent access
/// to the underlying block's methods:
///
/// ```ignore
/// let owned_typed_block = OwnedTypedBlock::new(block);
/// let span = owned_typed_block.span();  // Calls Block::span() via Deref
/// ```
#[derive(Debug, Clone)]
pub struct OwnedTypedBlock {
    /// The parsed construct type, or the original string if parsing failed
    pub construct_type: Result<ConstructType, String>,
    /// The underlying HCL block (owned)
    block: Block,
}

impl OwnedTypedBlock {
    /// Creates a new `OwnedTypedBlock` by parsing the block's identifier.
    ///
    /// The construct type is determined once at creation time.
    /// The block is owned by this struct.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let owned_typed_block = OwnedTypedBlock::new(block);
    /// assert!(owned_typed_block.construct_type.is_ok());
    /// ```
    pub fn new(block: Block) -> Self {
        let construct_type = ConstructType::from_str(block.ident.value().as_str())
            .map_err(|_| block.ident.value().as_str().to_string());

        Self {
            construct_type,
            block,
        }
    }

    /// Returns the construct type identifier as a string.
    ///
    /// For successfully parsed construct types (Action, Variable, etc.),
    /// returns the canonical lowercase representation.
    /// For unknown or invalid construct types, returns the original
    /// identifier string that failed to parse.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let owned_typed_block = OwnedTypedBlock::new(block);
    /// assert_eq!(owned_typed_block.ident_str(), "action");
    /// ```
    pub fn ident_str(&self) -> &str {
        match &self.construct_type {
            Ok(ct) => ct.as_ref(),
            Err(s) => s.as_str(),
        }
    }

    /// Clones the inner block.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let owned_typed_block = OwnedTypedBlock::new(block);
    /// let block_clone = owned_typed_block.clone_inner();
    /// ```
    pub fn clone_inner(&self) -> Block {
        self.block.clone()
    }

    /// Returns a reference to the inner block.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let owned_typed_block = OwnedTypedBlock::new(block);
    /// let block_ref = owned_typed_block.inner();
    /// ```
    pub fn inner(&self) -> &Block {
        &self.block
    }

    /// Consumes self and returns the inner block.
    ///
    /// This is a zero-cost operation that transfers ownership.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let owned_typed_block = OwnedTypedBlock::new(block);
    /// let block = owned_typed_block.into_inner();
    /// ```
    pub fn into_inner(self) -> Block {
        self.block
    }

    /// Extracts the first label as a string reference.
    ///
    /// Returns `None` if there are no labels or the first label is not a string.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let owned_typed_block = OwnedTypedBlock::new(block);
    /// if let Some(name) = owned_typed_block.first_label_string() {
    ///     println!("First label: {}", name);
    /// }
    /// ```
    pub fn first_label_string(&self) -> Option<&str> {
        self.label_string(0)
    }

    /// Extracts a label at the given index as a string reference.
    ///
    /// Returns `None` if the index is out of bounds or the label is not a string.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let owned_typed_block = OwnedTypedBlock::new(block);
    /// if let (Some(first), Some(second)) =
    ///     (owned_typed_block.label_string(0), owned_typed_block.label_string(1)) {
    ///     println!("Labels: {}, {}", first, second);
    /// }
    /// ```
    pub fn label_string(&self, index: usize) -> Option<&str> {
        self.block.labels.get(index).and_then(|l| match l {
            BlockLabel::String(s) => Some(s.value().as_str()),
            _ => None,
        })
    }

    /// Extracts all string labels as a Vec.
    ///
    /// Non-string labels are filtered out.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let owned_typed_block = OwnedTypedBlock::new(block);
    /// let labels = owned_typed_block.string_labels();
    /// ```
    pub fn string_labels(&self) -> Vec<&str> {
        self.block
            .labels
            .iter()
            .filter_map(|l| match l {
                BlockLabel::String(s) => Some(s.value().as_str()),
                _ => None,
            })
            .collect()
    }
}

/// Allow transparent access to the inner `Block`'s methods.
impl std::ops::Deref for OwnedTypedBlock {
    type Target = Block;

    fn deref(&self) -> &Self::Target {
        &self.block
    }
}
