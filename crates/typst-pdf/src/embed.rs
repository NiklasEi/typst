use crate::catalog::{document_date, pdf_date};
use crate::{PdfChunk, WithGlobalRefs};
use ecow::EcoString;
use pdf_writer::{Finish, Name, Ref, Str, TextStr};
use std::collections::HashMap;
use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::pdf::embed::EmbedElem;

/// Query for all [`EmbedElem`] and write them and their file specifications.
///
/// This returns a map of embedding names and references so that we can later add them to the
/// catalog's name dictionary.
pub fn write_embedded_files(
    ctx: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<EcoString, Ref>)> {
    let mut chunk = PdfChunk::new();

    let elements = ctx.document.introspector.query(&EmbedElem::elem().select());
    if !ctx.options.standards.embedded_files {
        if let Some(element) = elements.first() {
            bail!(
                element.span(),
                "file embeddings are currently only supported for PDF/A-3"
            );
        }
    }

    let mut embedded_files = HashMap::default();
    for elem in elements.iter() {
        let embed = elem.to_packed::<EmbedElem>().unwrap();
        let name = embed
            .name(StyleChain::default())
            .as_ref()
            .unwrap_or(&embed.resolved_path);
        embedded_files.insert(name.clone(), embed_file(ctx, &mut chunk, embed));
    }

    Ok((chunk, embedded_files))
}

/// Write the embedded file stream and its file specification.
fn embed_file(
    ctx: &WithGlobalRefs,
    chunk: &mut PdfChunk,
    embed: &Packed<EmbedElem>,
) -> Ref {
    let embedded_file_stream_ref = chunk.alloc.bump();
    let file_spec_dict_ref = chunk.alloc.bump();

    let length = embed.data().as_slice().len();

    let mut embedded_file =
        chunk.embedded_file(embedded_file_stream_ref, embed.data().as_slice());
    embedded_file.pair(Name(b"Length"), length as i32);
    if let Some(mime_type) = embed.mime_type(StyleChain::default()) {
        embedded_file.subtype(Name(mime_type.as_bytes()));
    }
    let (date, tz) = document_date(ctx.document.info.date, ctx.options.timestamp);
    if let Some(pdf_date) = date.and_then(|date| pdf_date(date, tz)) {
        embedded_file.params().modification_date(pdf_date).finish();
    }
    embedded_file.finish();

    let path = embed.resolved_path().replace("\\", "/");
    let mut file_spec = chunk.file_spec(file_spec_dict_ref);
    file_spec
        .path(Str(path.as_bytes()))
        .unic_file(TextStr(&path))
        .insert(Name(b"EF"))
        .dict()
        .pair(Name(b"F"), embedded_file_stream_ref)
        .pair(Name(b"UF"), embedded_file_stream_ref);
    if let Some(relationship) = embed.relationship(StyleChain::default()) {
        if ctx.options.standards.pdfa {
            let name = relationship.name();
            // PDF 2.0, but ISO 19005-3 (PDF/A-3) Annex E allows it for PDF/A-3
            file_spec.pair(Name(b"AFRelationship"), Name(name.as_bytes()));
        }
    }
    if let Some(description) = embed.description(StyleChain::default()) {
        file_spec.description(TextStr(description));
    }

    file_spec_dict_ref
}
