//! Bounded local document conversion. Uploaded documents are parsed as data;
//! Office, document macros, external relationships, and shell text never execute.

use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_UPLOAD: usize = 12 * 1024 * 1024;
const MAX_TEXT: usize = 180 * 1024;
const MAX_EXPORT: usize = 64 * 1024;
static NEXT_IMPORT: AtomicU64 = AtomicU64::new(0);

fn decode_base64(text: &str) -> Result<Vec<u8>, String> {
    if text.len() > MAX_UPLOAD.div_ceil(3) * 4 || !text.len().is_multiple_of(4) {
        return Err("Upload exceeds 12 MiB or contains invalid base64.".into());
    }
    fn digit(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let invalid = || "The uploaded file contains invalid base64.".to_owned();
    let mut bytes = Vec::with_capacity(text.len() / 4 * 3);
    for (index, quartet) in text.as_bytes().as_chunks::<4>().0.iter().enumerate() {
        let last = (index + 1) * 4 == text.len();
        let a = digit(quartet[0]).ok_or_else(invalid)?;
        let b = digit(quartet[1]).ok_or_else(invalid)?;
        let c = if quartet[2] == b'=' {
            if !last || quartet[3] != b'=' || b & 15 != 0 {
                return Err(invalid());
            }
            0
        } else {
            digit(quartet[2]).ok_or_else(invalid)?
        };
        let d = if quartet[3] == b'=' {
            if !last || c & 3 != 0 {
                return Err(invalid());
            }
            0
        } else {
            digit(quartet[3]).ok_or_else(invalid)?
        };
        bytes.push(a << 2 | b >> 4);
        if quartet[2] != b'=' {
            bytes.push(b << 4 | c >> 2);
        }
        if quartet[3] != b'=' {
            bytes.push(c << 6 | d);
        }
    }
    if bytes.len() > MAX_UPLOAD {
        return Err("Upload exceeds 12 MiB.".into());
    }
    Ok(bytes)
}

fn upload_name(body: &Value) -> Result<(String, String), String> {
    let name = body["name"]
        .as_str()
        .filter(|name| !name.trim().is_empty() && name.len() <= 240)
        .ok_or("Choose a file with a name shorter than 240 bytes.")?;
    if name.chars().any(char::is_control) {
        return Err("The file name contains unsupported characters.".into());
    }
    // Names are labels only. No uploaded name is joined to a filesystem path.
    let label = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let (stem, extension) = label
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.trim().is_empty())
        .ok_or("Choose a text, Office, PDF, screenshot, or captioned video file.")?;
    let format = extension.to_ascii_lowercase();
    if ![
        "txt", "md", "csv", "tsv", "srt", "vtt", "docx", "pptx", "xlsx", "pdf", "png", "jpg",
        "jpeg", "bmp", "mp4", "webm", "mov", "mkv",
    ]
    .contains(&format.as_str())
    {
        return Err("Choose TXT, MD, CSV, TSV, SRT, VTT, DOCX, PPTX, XLSX, PDF, PNG, JPG, BMP, or a captioned video. Macro-enabled Office files are not supported.".into());
    }
    Ok((stem.trim().to_owned(), format))
}

struct TemporaryImport(PathBuf);

impl Drop for TemporaryImport {
    fn drop(&mut self) {
        for name in ["input", "helper.ps1", "output.json"] {
            let _ = fs::remove_file(self.0.join(name));
        }
        let _ = fs::remove_dir(&self.0);
    }
}

fn private_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "Cannot prepare local document conversion.")?;
    file.write_all(bytes)
        .map_err(|_| "Cannot prepare local document conversion.".into())
}

fn converted_text(directory: &Path, bytes: &[u8], format: &str) -> Result<String, String> {
    if !cfg!(windows) {
        return Err("Office, PDF, screenshot and video conversion currently requires Windows. Import a text or caption file, or paste the document text on this platform.".into());
    }
    let parent = fs::canonicalize(directory).map_err(|_| "Cannot resolve local import storage.")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let folder = parent.join(format!(
        ".document-import-{}-{stamp}-{}",
        std::process::id(),
        NEXT_IMPORT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&folder).map_err(|_| "Cannot prepare local import storage.")?;
    let temporary = TemporaryImport(folder);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary.0, fs::Permissions::from_mode(0o700))
            .map_err(|_| "Cannot protect local import storage.")?;
    }
    let input = temporary.0.join("input");
    let helper = temporary.0.join("helper.ps1");
    let output = temporary.0.join("output.json");
    private_write(&input, bytes)?;
    private_write(&helper, include_bytes!("../scripts/document-import.ps1"))?;
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(&helper)
        .arg("-InputFile")
        .arg(&input)
        .arg("-OutputFile")
        .arg(&output)
        .arg("-Format")
        .arg(format)
        .env_remove("GROQ_API_KEY")
        .env_remove("NVIDIA_API_KEY")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|_| "Cannot start Windows document import. Paste the text instead.")?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < Duration::from_secs(90) => {
                thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "Document conversion timed out. Split the file or paste its text.".into(),
                );
            }
        }
    };
    let metadata = fs::metadata(&output)
        .map_err(|_| "Windows document import could not read this file. Paste the text instead.")?;
    if metadata.len() > 2 * 1024 * 1024 {
        return Err("The converted document exceeds the supported size.".into());
    }
    let result: Value = serde_json::from_slice(
        &fs::read(&output).map_err(|_| "Cannot read the converted document.")?,
    )
    .map_err(|_| "Windows document import returned invalid data.")?;
    if !status.success() {
        return Err(result["error"]
            .as_str()
            .unwrap_or("The file could not be read. Paste its text instead.")
            .to_owned());
    }
    result["text"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "The converted document contains no text.".into())
}

/// Extract a bounded local preview. The caller decides when to save the text as notes.
pub fn import(directory: &Path, body: &Value) -> Result<Value, String> {
    let (title, format) = upload_name(body)?;
    let encoded = body["content_base64"]
        .as_str()
        .ok_or("Choose a file to import.")?;
    let bytes = decode_base64(encoded)?;
    if bytes.is_empty() {
        return Err("The selected file is empty.".into());
    }
    if ["mp4", "webm", "mov", "mkv"].contains(&format.as_str()) {
        if let Some(value) = body.get("transcript") {
            let transcript = value
                .as_str()
                .filter(|text| text.len() <= MAX_TEXT)
                .map(str::trim)
                .filter(|text| {
                    !text.is_empty()
                        && !text.contains('\0')
                        && text.chars().any(char::is_alphanumeric)
                })
                .ok_or("Provide a readable video transcript within 180 KiB.")?;
            return Ok(
                json!({"title":title,"text":transcript,"format":format,"method":"supplied transcript"}),
            );
        }
    }
    let text = if ["txt", "md", "csv", "tsv", "srt", "vtt"].contains(&format.as_str()) {
        String::from_utf8(bytes)
            .map_err(|_| "Text files must use UTF-8. Resave the file as UTF-8 or paste its text.")?
    } else {
        converted_text(directory, &bytes, &format)?
    };
    let text = text.trim_start_matches('\u{feff}').trim();
    if text.is_empty() || text.len() > MAX_TEXT || text.contains('\0') {
        return Err("Import readable text within 180 KiB. Split larger documents first.".into());
    }
    let method = match format.as_str() {
        "txt" | "md" | "csv" | "tsv" | "srt" | "vtt" => "UTF-8 text",
        "docx" | "pptx" | "xlsx" => "Office document text; review extracted text before saving",
        "mp4" | "webm" | "mov" | "mkv" => {
            "Embedded captions only; speech and video frames are not interpreted"
        }
        _ => "Windows OCR; review extracted text before saving",
    };
    Ok(json!({"title":title,"text":text,"format":format,"method":method}))
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn field<'a>(body: &'a Value, name: &str, maximum: usize) -> Result<&'a str, String> {
    let text = body[name]
        .as_str()
        .filter(|text| !text.trim().is_empty() && text.len() <= maximum)
        .ok_or_else(|| format!("Document {name} is missing or exceeds the supported length."))?;
    if text.chars().any(|character| {
        (character.is_control() && !['\n', '\r', '\t'].contains(&character))
            || ['\u{fffe}', '\u{ffff}'].contains(&character)
    }) {
        return Err("Document text contains unsupported control characters.".into());
    }
    Ok(text)
}

fn paragraph(text: &str, style: &str) -> String {
    let properties = if style.is_empty() {
        String::new()
    } else {
        format!("<w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr>")
    };
    format!(
        "<w:p>{properties}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml(text)
    )
}

fn diagram(diagram: &Value) -> Result<(String, Vec<String>, u32), String> {
    let title = field(diagram, "title", 240)?;
    let nodes = diagram["nodes"]
        .as_array()
        .filter(|nodes| !nodes.is_empty() && nodes.len() <= 8)
        .ok_or("A diagram needs between 1 and 8 nodes.")?;
    let labels: Vec<&str> = nodes
        .iter()
        .map(|node| {
            let wrapper = json!({"label":node});
            field(&wrapper, "label", 120).map(|_| node.as_str().unwrap())
        })
        .collect::<Result<_, _>>()?;
    let edges = diagram["edges"]
        .as_array()
        .filter(|edges| edges.len() <= 16)
        .ok_or("A diagram supports at most 16 connections.")?;
    let rows = nodes.len().div_ceil(2);
    let height = 120 + rows as u32 * 150;
    let position = |index: usize| ((index % 2) as u32 * 570 + 30, (index / 2) as u32 * 150 + 90);
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"{height}\" viewBox=\"0 0 1200 {height}\"><rect width=\"1200\" height=\"{height}\" fill=\"#ffffff\"/><defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path d=\"M0 0L10 5L0 10Z\" fill=\"#326d60\"/></marker></defs><text x=\"30\" y=\"45\" font-family=\"Arial,sans-serif\" font-size=\"28\" fill=\"#163d35\">{}</text>", xml(title));
    let mut descriptions: Vec<String> = labels
        .iter()
        .map(|label| format!("Diagram node: {label}."))
        .collect();
    for edge in edges {
        let from = edge["from"]
            .as_u64()
            .filter(|from| (*from as usize) < labels.len())
            .ok_or("A diagram connection refers to a missing node.")? as usize;
        let to = edge["to"]
            .as_u64()
            .filter(|to| (*to as usize) < labels.len())
            .ok_or("A diagram connection refers to a missing node.")? as usize;
        if from == to {
            return Err("Diagram connections must join different nodes.".into());
        }
        let (start_x, start_y) = position(from);
        let (end_x, end_y) = position(to);
        let (x1, y1, x2, y2) = if from / 2 == to / 2 {
            if from < to {
                (start_x + 510, start_y + 45, end_x - 8, end_y + 45)
            } else {
                (start_x, start_y + 45, end_x + 518, end_y + 45)
            }
        } else if from < to {
            (start_x + 255, start_y + 90, end_x + 255, end_y - 8)
        } else {
            (start_x + 255, start_y, end_x + 255, end_y + 98)
        };
        svg.push_str(&format!("<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#326d60\" stroke-width=\"3\" marker-end=\"url(#arrow)\"/>"));
        descriptions.push(format!("{} connects to {}.", labels[from], labels[to]));
    }
    for (index, label) in labels.iter().enumerate() {
        let (x, y) = position(index);
        svg.push_str(&format!("<rect x=\"{x}\" y=\"{y}\" width=\"510\" height=\"90\" rx=\"12\" fill=\"#edf8f4\" stroke=\"#326d60\" stroke-width=\"2\"/><text x=\"{}\" y=\"{}\" font-family=\"Arial,sans-serif\" font-size=\"22\" fill=\"#163d35\">", x + 20, y + 35));
        let mut lines = vec![String::new()];
        for word in label.split_whitespace() {
            if lines.last().unwrap().chars().count() + word.chars().count() > 40 && lines.len() < 3
            {
                lines.push(String::new());
            }
            let line = lines.last_mut().unwrap();
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        for (line_index, line) in lines.iter().enumerate() {
            svg.push_str(&format!(
                "<tspan x=\"{}\" dy=\"{}\">{}</tspan>",
                x + 20,
                if line_index == 0 { 0 } else { 24 },
                xml(line)
            ));
        }
        svg.push_str("</text>");
    }
    svg.push_str("</svg>");
    Ok((svg, descriptions, height))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn zip(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    fn word(output: &mut Vec<u8>, value: u16) {
        output.extend(value.to_le_bytes());
    }
    fn number(output: &mut Vec<u8>, value: u32) {
        output.extend(value.to_le_bytes());
    }
    let mut output = Vec::new();
    let mut central = Vec::new();
    let count = entries.len() as u16;
    for (name, content) in entries {
        let offset = output.len() as u32;
        let length = content.len() as u32;
        let checksum = crc32(&content);
        number(&mut output, 0x04034b50);
        for value in [20, 0x0800, 0, 0, 0] {
            word(&mut output, value);
        }
        for value in [checksum, length, length] {
            number(&mut output, value);
        }
        word(&mut output, name.len() as u16);
        word(&mut output, 0);
        output.extend(name.as_bytes());
        output.extend(content);
        number(&mut central, 0x02014b50);
        for value in [20, 20, 0x0800, 0, 0, 0] {
            word(&mut central, value);
        }
        for value in [checksum, length, length] {
            number(&mut central, value);
        }
        word(&mut central, name.len() as u16);
        for _ in 0..4 {
            word(&mut central, 0);
        }
        number(&mut central, 0);
        number(&mut central, offset);
        central.extend(name.as_bytes());
    }
    let central_offset = output.len() as u32;
    let central_length = central.len() as u32;
    output.extend(central);
    number(&mut output, 0x06054b50);
    for value in [0, 0, count, count] {
        word(&mut output, value);
    }
    number(&mut output, central_length);
    number(&mut output, central_offset);
    word(&mut output, 0);
    output
}

/// Create an editable OOXML document, including a vector diagram and its text equivalent.
pub fn docx(body: &Value) -> Result<Vec<u8>, String> {
    if body.to_string().len() > MAX_EXPORT {
        return Err("Export a document within 64 KiB of text.".into());
    }
    let title = field(body, "title", 240)?;
    let card_sections;
    let sections = if let Some(sections) = body["sections"]
        .as_array()
        .filter(|sections| !sections.is_empty() && sections.len() <= 32)
    {
        sections
    } else if let Some(cards) = body["cards"]
        .as_array()
        .filter(|cards| !cards.is_empty() && cards.len() <= 32)
    {
        card_sections = cards.iter().enumerate().map(|(index, card)| {
            let front = field(card, "front", 2000)?;
            let back = field(card, "back", 6000)?;
            Ok(json!({"heading":format!("Card {}",index+1),"text":format!("Question\n{front}\n\nAnswer\n{back}")}))
        }).collect::<Result<Vec<Value>,String>>()?;
        &card_sections
    } else {
        return Err("Export between 1 and 32 study note sections or flashcards.".into());
    };
    let mut content = paragraph(title, "Title");
    for section in sections {
        content.push_str(&paragraph(field(section, "heading", 240)?, "Heading1"));
        for line in field(section, "text", MAX_EXPORT)?.lines() {
            content.push_str(&paragraph(line, ""));
        }
    }
    let mut image = None;
    if let Some(value) = body.get("diagram").filter(|value| !value.is_null()) {
        let (svg, descriptions, height) = diagram(value)?;
        content.push_str(&paragraph(field(value, "title", 240)?, "Heading1"));
        let image_height = u64::from(height) * 5_943_600 / 1200;
        content.push_str(&format!("<w:p><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"5943600\" cy=\"{image_height}\"/><wp:docPr id=\"1\" name=\"Study diagram\" descr=\"{}\"/><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"1\" name=\"diagram.svg\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"rIdDiagram\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"5943600\" cy=\"{image_height}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>", xml(field(value, "title", 240)?)));
        content.push_str(&paragraph("Diagram connections", "Heading2"));
        for description in descriptions {
            content.push_str(&paragraph(&description, ""));
        }
        image = Some(svg.into_bytes());
    }
    if let Some(citations) = body.get("citations").and_then(Value::as_array) {
        if citations.len() > 64 {
            return Err("Export at most 64 source quotations.".into());
        }
        if !citations.is_empty() {
            content.push_str(&paragraph("Sources", "Heading1"));
        }
        for citation in citations {
            let id = field(citation, "source_id", 100)?;
            let quote = field(citation, "quote", MAX_EXPORT)?;
            content.push_str(&paragraph(&format!("{id}: {quote}"), ""));
        }
    }
    let document = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><w:body>{content}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1080\" w:right=\"1080\" w:bottom=\"1080\" w:left=\"1080\"/></w:sectPr></w:body></w:document>");
    let image_relationship = if image.is_some() {
        "<Relationship Id=\"rIdDiagram\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/diagram.svg\"/>"
    } else {
        ""
    };
    let mut entries = vec![
        ("[Content_Types].xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"svg\" ContentType=\"image/svg+xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>".to_vec()),
        ("_rels/.rels", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdDocument\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>".to_vec()),
        ("word/document.xml", document.into_bytes()),
        ("word/_rels/document.xml.rels", format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>{image_relationship}</Relationships>").into_bytes()),
        ("word/styles.xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\"/><w:sz w:val=\"22\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"160\"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style><w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:basedOn w:val=\"Normal\"/><w:rPr><w:b/><w:sz w:val=\"40\"/><w:color w:val=\"163D35\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:name w:val=\"heading 1\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:keepNext/><w:spacing w:before=\"280\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"28\"/><w:color w:val=\"163D35\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading2\"><w:name w:val=\"heading 2\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:keepNext/></w:pPr><w:rPr><w:b/><w:sz w:val=\"24\"/></w:rPr></w:style></w:styles>".to_vec()),
    ];
    if let Some(image) = image {
        entries.push(("word/media/diagram.svg", image));
    }
    Ok(zip(entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let a = chunk[0];
            let b = chunk.get(1).copied().unwrap_or(0);
            let c = chunk.get(2).copied().unwrap_or(0);
            output.push(ALPHABET[(a >> 2) as usize] as char);
            output.push(ALPHABET[((a & 3) << 4 | b >> 4) as usize] as char);
            output.push(if chunk.len() > 1 {
                ALPHABET[((b & 15) << 2 | c >> 6) as usize] as char
            } else {
                '='
            });
            output.push(if chunk.len() > 2 {
                ALPHABET[(c & 63) as usize] as char
            } else {
                '='
            });
        }
        output
    }

    #[test]
    fn base64_rejects_bad_padding_invalid_bytes_and_large_uploads() {
        for text in [
            "A", "A===", "AA=A", "AAAA====", "AB==", "AAB=", "AA!A", "AA\nA",
        ] {
            assert!(decode_base64(text).is_err(), "accepted {text}");
        }
        for bytes in [
            b"a".as_slice(),
            b"ab",
            b"abc",
            "Unicode \u{4e2d}\u{6587}".as_bytes(),
        ] {
            assert_eq!(decode_base64(&encoded(bytes)).unwrap(), bytes);
        }
        assert!(decode_base64(&"A".repeat(MAX_UPLOAD.div_ceil(3) * 4 + 4)).is_err());
    }

    #[test]
    fn text_import_uses_filename_only_as_metadata_and_requires_readable_utf8() {
        let value = import(Path::new("this-path-does-not-exist"), &json!({"name":"../../outside/lesson.md","content_base64":encoded("\u{feff}Notes about unit tests.".as_bytes())})).unwrap();
        assert_eq!(value["title"], "lesson");
        assert_eq!(value["text"], "Notes about unit tests.");
        assert!(import(
            Path::new("."),
            &json!({"name":"unsafe.docm","content_base64":"YQ=="})
        )
        .is_err());
        assert!(import(
            Path::new("."),
            &json!({"name":"binary.txt","content_base64":"/w=="})
        )
        .is_err());
        assert!(import(
            Path::new("."),
            &json!({"name":"empty.txt","content_base64":"IA=="})
        )
        .is_err());
        assert!(import(
            Path::new("."),
            &json!({"name":"large.txt","content_base64":encoded(&vec![b'a';MAX_TEXT+1])})
        )
        .is_err());
    }

    #[test]
    fn supplied_video_transcript_skips_conversion_and_enforces_upload_and_text_limits() {
        let directory = Path::new("this-conversion-directory-does-not-exist");
        let body = json!({"name":"lecture.mp4","content_base64":encoded(b"video fixture"),"transcript":"  A unit test checks one function in isolation.  "});
        let imported = import(directory, &body).unwrap();
        assert_eq!(
            imported["text"],
            "A unit test checks one function in isolation."
        );
        assert_eq!(imported["format"], "mp4");
        assert_eq!(imported["method"], "supplied transcript");
        for transcript in [
            json!("x".repeat(MAX_TEXT + 1)),
            json!("   "),
            json!("\u{0}hidden"),
            json!(42),
        ] {
            let mut invalid = body.clone();
            invalid["transcript"] = transcript;
            assert!(import(directory, &invalid).is_err());
        }
        let mut invalid = body.clone();
        invalid["content_base64"] = json!("A".repeat(MAX_UPLOAD.div_ceil(3) * 4 + 4));
        assert!(import(directory, &invalid).is_err());
        invalid["content_base64"] = json!("");
        assert!(import(directory, &invalid).is_err());
    }

    fn sample() -> Value {
        json!({"title":"Tests & evidence","sections":[{"heading":"Unit tests < integration tests","text":"A unit test checks one function in isolation.\nUnicode: \u{4e2d}\u{6587}."}],"diagram":{"title":"Testing relationship","nodes":["Unit test","Integration test"],"edges":[{"from":0,"to":1}]},"citations":[{"source_id":"S3","quote":"A unit test checks one function in isolation."}]})
    }

    #[test]
    fn word_export_contains_editable_text_vector_diagram_and_no_active_content() {
        let bytes = docx(&sample()).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        let text = String::from_utf8_lossy(&bytes);
        for expected in [
            "Tests &amp; evidence",
            "Unit tests &lt; integration tests",
            "word/media/diagram.svg",
            "http://schemas.openxmlformats.org/drawingml/2006/picture",
            "Unit test connects to Integration test.",
            "Unicode: \u{4e2d}\u{6587}.",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
        assert!(!text.contains("TargetMode=\"External\""));
        assert!(!text.contains("vbaProject"));
        if let Ok(path) = std::env::var("STUDY_COACH_DOCUMENT_EVIDENCE") {
            fs::write(path, bytes).unwrap();
        }
    }

    #[test]
    fn word_export_rejects_missing_text_oversized_fields_and_invalid_diagram_links() {
        assert!(docx(&json!({"title":"No body","sections":[]})).is_err());
        let mut value = sample();
        value["diagram"]["edges"][0]["to"] = json!(20);
        assert!(docx(&value).is_err());
        value = sample();
        value["diagram"]["edges"][0]["to"] = json!(0);
        assert!(docx(&value).is_err());
        value = sample();
        value["sections"][0]["text"] = json!("x".repeat(MAX_EXPORT));
        assert!(docx(&value).is_err());
        value = sample();
        value["title"] = json!("Bad\u{1}text");
        assert!(docx(&value).is_err());
    }

    #[test]
    fn flashcard_export_creates_editable_question_and_answer_sections() {
        let bytes = docx(&json!({"title":"Recall practice","kind":"flashcards","sections":[],"cards":[{"front":"What does a unit test check?","back":"One function in isolation."}]})).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        for expected in [
            "Card 1",
            "What does a unit test check?",
            "One function in isolation.",
        ] {
            assert!(text.contains(expected));
        }
        assert!(docx(
            &json!({"title":"Incomplete","sections":[],"cards":[{"front":"Missing answer"}]})
        )
        .is_err());
    }

    #[test]
    fn crc_matches_known_zip_checksum() {
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
    }
}
