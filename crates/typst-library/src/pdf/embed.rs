use ecow::EcoString;
use typst_syntax::{Span, Spanned};

use crate::diag::{At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, func, scope, Cast, Content, NativeElement, Packed, Show, StyleChain,
};
use crate::introspection::Locatable;
use crate::loading::Readable;
use crate::World;

/// A file that will be embedded into the output PDF.
///
/// This can be used to distribute additional files that are related to the PDF
/// within it. PDF readers will display the files in a file listing.
///
/// Some international standards use this mechanism to embed machine-readable
/// data (e.g., ZUGFeRD/Factur-X for invoices) that mirrors the visual content
/// of the PDF.
///
/// # Example
/// ```typ
/// #pdf.embed(
///   "experiment.csv",
///   relationship: "supplement",
///   mime-type: "text/csv",
///   description: "Raw Oxygen readings from the Arctic experiment",
/// )
/// ```
///
/// # Notes
/// - This element is ignored if exporting to a format other than PDF.
/// - File embeddings are not currently supported for PDF/A-2, even if the
///   embedded file conforms to PDF/A-1 or PDF/A-2.
#[elem(scope, Show, Locatable)]
pub struct EmbedElem {
    /// Path to a file to be embedded.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    #[required]
    #[parse(
        let Spanned { v: path, span } =
        args.expect::<Spanned<EcoString>>("path to the file to be embedded")?;
        let id = span.resolve_path(&path).at(span)?;
        let data = engine.world.file(id).at(span)?;
        path
    )]
    #[borrowed]
    pub path: EcoString,

    /// The resolved project-relative path.
    #[internal]
    #[required]
    #[parse(id.vpath().as_rootless_path().to_string_lossy().replace("\\", "/").into())]
    pub resolved_path: EcoString,

    /// The raw file data.
    #[internal]
    #[required]
    #[parse(Readable::Bytes(data))]
    pub data: Readable,

    /// The relationship of the embedded file to the document.
    ///
    /// Ignored if export doesn't target PDF/A-3.
    pub relationship: Option<EmbeddedFileRelationship>,

    /// The MIME type of the embedded file.
    #[borrowed]
    pub mime_type: Option<EcoString>,

    /// A description for the embedded file.
    #[borrowed]
    pub description: Option<EcoString>,
}

#[scope]
impl EmbedElem {
    /// Decode a file embedding from bytes or a string.
    #[func(title = "Embed Data")]
    fn decode(
        /// The call span of this function.
        span: Span,
        /// The path that will be written into the PDF. Typst will not read from
        /// this path since the data is provided in the following argument.
        path: EcoString,
        /// The data to embed as a file.
        data: Readable,
        /// The relationship of the embedded file to the document.
        #[named]
        relationship: Option<Option<EmbeddedFileRelationship>>,
        /// The MIME type of the embedded file.
        #[named]
        mime_type: Option<Option<EcoString>>,
        /// A description for the embedded file.
        #[named]
        description: Option<Option<EcoString>>,
    ) -> StrResult<Content> {
        let mut elem = EmbedElem::new(path.clone(), path, data);
        if let Some(description) = description {
            elem.push_description(description);
        }
        if let Some(mime_type) = mime_type {
            elem.push_mime_type(mime_type);
        }
        if let Some(relationship) = relationship {
            elem.push_relationship(relationship);
        }
        Ok(elem.pack().spanned(span))
    }
}

impl Show for Packed<EmbedElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}

/// The relationship of an embedded file with the document.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum EmbeddedFileRelationship {
    /// The PDF document was created from the source file.
    Source,
    /// The file was used to derive a visual presentation in the PDF.
    Data,
    /// An alternative representation of the document.
    Alternative,
    /// Additional resources for the document.
    Supplement,
}
