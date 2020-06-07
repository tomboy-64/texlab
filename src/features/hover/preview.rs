use crate::{
    features::prelude::*,
    syntax::{CharStream, LatexIncludeKind},
    tex::{CompileError, CompileParams, DistributionKind, Format},
};
use image::{png::PNGEncoder, ColorType, DynamicImage, GenericImageView, ImageBuffer, RgbaImage};
use log::warn;
use std::io::{self, Cursor};
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use thiserror::Error;
use tokio::process::Command;

const PREVIEW_ENVIRONMENTS: &[&str] = &[
    "align",
    "alignat",
    "aligned",
    "alignedat",
    "algorithmic",
    "array",
    "Bmatrix",
    "bmatrix",
    "cases",
    "CD",
    "eqnarray",
    "equation",
    "gather",
    "gathered",
    "matrix",
    "multline",
    "pmatrix",
    "smallmatrix",
    "split",
    "subarray",
    "Vmatrix",
    "vmatrix",
];

const IGNORED_PACKAGES: &[&str] = &["biblatex", "pgf", "tikz"];

#[derive(Debug, Clone, Copy)]
enum MathElement {
    Environment(latex::Environment),
    Equation(latex::Equation),
    Inline(latex::Inline),
}

impl MathElement {
    fn range(&self, tree: &latex::Tree) -> Range {
        match self {
            Self::Environment(env) => env.range(tree),
            Self::Equation(eq) => eq.range(tree),
            Self::Inline(inline) => inline.range(tree),
        }
    }
}

#[derive(Debug, Error)]
enum RenderError {
    #[error("an I/O error occurred: `{0}`")]
    IO(#[from] io::Error),
    #[error("a compile error occurred: `{0}`")]
    Compile(#[from] CompileError),
    #[error("compilation failed")]
    DviNotFound,
    #[error("dvipng is not installed")]
    DviPngNotInstalled,
    #[error("calling dvipng failed")]
    DviPngFaulty,
    #[error("failed to decode image")]
    DecodeImage,
}

fn is_preview_environment(
    ctx: &FeatureContext<HoverParams>,
    table: &latex::SymbolTable,
    environment: latex::Environment,
) -> bool {
    let canonical_name = environment
        .left
        .name(&table)
        .map(latex::Token::text)
        .unwrap_or_default()
        .replace('*', "");

    PREVIEW_ENVIRONMENTS.contains(&canonical_name.as_ref())
        || theorem_environments(ctx).contains(&canonical_name.as_ref())
}

fn theorem_environments(ctx: &FeatureContext<HoverParams>) -> Vec<&str> {
    let mut names = Vec::new();
    for doc in ctx.related() {
        if let DocumentContent::Latex(table) = &doc.content {
            table
                .theorem_definitions
                .iter()
                .map(|thm| thm.name(&table).text())
                .for_each(|thm| names.push(thm));
        }
    }
    names
}

async fn render(req: &FeatureContext<HoverParams>, range: Range) -> Result<Hover, RenderError> {
    let code = generate_code(req, range);
    let params = CompileParams {
        file_name: "preview.tex",
        code: &code,
        format: Format::Latex,
        timeout: Duration::from_secs(10),
    };
    let dir = req.distro.compile(params).await?.dir;
    if !dir.path().join("preview.dvi").exists() {
        return Err(RenderError::DviNotFound);
    }

    let img = add_margin(dvipng(&dir).await?);
    let base64 = encode_image(img);
    let markdown = format!("![preview](data:image/png;base64,{})", base64);
    dir.close()?;
    Ok(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(range),
    })
}

fn generate_code(req: &FeatureContext<HoverParams>, range: Range) -> String {
    let mut code = String::new();
    code.push_str("\\documentclass{article}\n");
    code.push_str("\\thispagestyle{empty}\n");
    generate_includes(req, &mut code);
    generate_command_definitions(req, &mut code);
    generate_math_operators(req, &mut code);
    generate_theorem_definitions(req, &mut code);
    code.push_str("\\begin{document}\n");
    code.push_str(&CharStream::extract(&req.current().text, range));
    code.push('\n');
    code.push_str("\\end{document}\n");
    code
}

fn generate_includes(req: &FeatureContext<HoverParams>, code: &mut String) {
    for doc in req.related() {
        if let DocumentContent::Latex(table) = &doc.content {
            let text = &doc.text;
            for include in &table.includes {
                if include.kind == LatexIncludeKind::Package {
                    if include
                        .paths(&table)
                        .iter()
                        .all(|path| IGNORED_PACKAGES.contains(&path.text()))
                    {
                        continue;
                    }

                    if include
                        .paths(&table)
                        .iter()
                        .map(|path| format!("{}.sty", path.text()))
                        .any(|name| !COMPONENT_DATABASE.exists(&name))
                    {
                        continue;
                    }

                    code.push_str(&CharStream::extract(&text, table[include.parent].range()));
                    code.push('\n');
                }
            }
        }
    }
}

fn generate_command_definitions(req: &FeatureContext<HoverParams>, code: &mut String) {
    for doc in req.related() {
        if let DocumentContent::Latex(table) = &doc.content {
            table
                .command_definitions
                .iter()
                .filter(|def| !def.definition_name(&table).contains("@"))
                .map(|def| CharStream::extract(&doc.text, table[def.parent].range()))
                .for_each(|def| {
                    code.push_str(&def);
                    code.push('\n');
                });
        }
    }
}

fn generate_math_operators(req: &FeatureContext<HoverParams>, code: &mut String) {
    for doc in req.related() {
        if let DocumentContent::Latex(table) = &doc.content {
            table
                .math_operators
                .iter()
                .filter(|op| !op.definition_name(&table).contains("@"))
                .map(|op| CharStream::extract(&doc.text, table[op.parent].range()))
                .for_each(|op| {
                    code.push_str(&op);
                    code.push('\n');
                });
        }
    }
}

fn generate_theorem_definitions(req: &FeatureContext<HoverParams>, code: &mut String) {
    for doc in req.related() {
        if let DocumentContent::Latex(table) = &doc.content {
            table
                .theorem_definitions
                .iter()
                .map(|thm| CharStream::extract(&doc.text, table[thm.parent].range()))
                .for_each(|thm| {
                    code.push_str(&thm);
                    code.push('\n');
                })
        }
    }
}

async fn dvipng(dir: &TempDir) -> Result<DynamicImage, RenderError> {
    let process = Command::new("dvipng")
        .args(&["-D", "175", "-T", "tight", "preview.dvi"])
        .current_dir(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| RenderError::DviPngNotInstalled)?;

    process.await.map_err(|_| RenderError::DviPngFaulty)?;

    let png_file = dir.path().join("preview1.png");
    let png = image::open(png_file).map_err(|_| RenderError::DecodeImage)?;
    Ok(png)
}

fn add_margin(image: DynamicImage) -> RgbaImage {
    let margin = 5;
    let width = image.width() + 2 * margin;
    let height = image.height() + 2 * margin;
    let mut result = ImageBuffer::from_pixel(width, height, image::Rgba([0xFF, 0xFF, 0xFF, 0xFF]));

    for x in 0..image.width() {
        for y in 0..image.height() {
            let pixel = image.get_pixel(x, y);
            result.put_pixel(x + margin, y + margin, pixel);
        }
    }
    result
}

fn encode_image(image: RgbaImage) -> String {
    let mut image_buf = Cursor::new(Vec::new());
    let png_encoder = PNGEncoder::new(&mut image_buf);
    let width = image.width();
    let height = image.height();
    png_encoder
        .encode(&image.into_raw(), width, height, ColorType::Rgba8)
        .unwrap();
    base64::encode(&image_buf.into_inner())
}

pub async fn hover_preview(ctx: &FeatureContext<HoverParams>) -> Option<Hover> {
    if !ctx.client_capabilities.has_hover_markdown_support()
        || ctx.distro.kind() == DistributionKind::Tectonic
    {
        return None;
    }

    if let DocumentContent::Latex(table) = &ctx.current().content {
        let mut elements = Vec::new();
        table
            .inlines
            .iter()
            .map(|inline| MathElement::Inline(*inline))
            .for_each(|inline| elements.push(inline));

        table
            .equations
            .iter()
            .map(|eq| MathElement::Equation(*eq))
            .for_each(|eq| elements.push(eq));

        table
            .environments
            .iter()
            .filter(|env| is_preview_environment(ctx, table, **env))
            .map(|env| MathElement::Environment(*env))
            .for_each(|env| elements.push(env));

        let pos = ctx.params.text_document_position_params.position;

        let range = elements
            .iter()
            .map(|elem| elem.range(&table))
            .find(|range| range.contains(pos))?;

        return match render(ctx, range).await {
            Ok(hover) => Some(hover),
            Err(why) => {
                warn!("Preview failed: {}", why);
                None
            }
        };
    }
    None
}
