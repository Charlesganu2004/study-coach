param(
    [Parameter(Mandatory = $true)][string]$InputFile,
    [Parameter(Mandatory = $true)][string]$OutputFile,
    [Parameter(Mandatory = $true)][string]$Format
)

$ErrorActionPreference = 'Stop'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$maximumText = 180 * 1024

function Read-XmlEntry($Entry) {
    if ($null -eq $Entry -or $Entry.Length -gt 2MB) { throw 'An Office document part is missing or exceeds 2 MiB.' }
    $settings = New-Object System.Xml.XmlReaderSettings
    $settings.DtdProcessing = [System.Xml.DtdProcessing]::Prohibit
    $settings.XmlResolver = $null
    $settings.MaxCharactersInDocument = 2MB
    $stream = $Entry.Open()
    try {
        $reader = [System.Xml.XmlReader]::Create($stream, $settings)
        try {
            $document = New-Object System.Xml.XmlDocument
            $document.XmlResolver = $null
            $document.Load($reader)
            return $document
        } finally { $reader.Dispose() }
    } finally { $stream.Dispose() }
}

function Read-OfficeText {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($InputFile)
    try {
        if ($archive.Entries.Count -gt 1000) { throw 'The Word file has too many archive entries.' }
        [long]$expanded = 0
        foreach ($entry in $archive.Entries) {
            $expanded += $entry.Length
            if ($entry.FullName -match '(^|[\\/])\.\.([\\/]|$)' -or $entry.FullName -match '^[/\\]' -or $entry.FullName -match '(?i)vbaProject\.bin$') {
                throw 'This Word archive contains unsupported entries.'
            }
        }
        if ($expanded -gt 32MB) { throw 'The expanded Word file exceeds 32 MiB.' }
        if ($Format -eq 'docx') {
            $document = Read-XmlEntry ($archive.GetEntry('word/document.xml'))
            $namespaces = New-Object System.Xml.XmlNamespaceManager($document.NameTable)
            $namespaces.AddNamespace('w', 'http://schemas.openxmlformats.org/wordprocessingml/2006/main')
            $paragraphs = foreach ($paragraph in $document.SelectNodes('//w:p', $namespaces)) {
                $parts = foreach ($node in $paragraph.SelectNodes('.//w:t | .//w:tab | .//w:br', $namespaces)) {
                    if ($node.LocalName -eq 't') { $node.InnerText }
                    elseif ($node.LocalName -eq 'tab') { "`t" }
                    else { "`n" }
                }
                $parts -join ''
            }
            return $paragraphs -join "`n"
        }
        if ($Format -eq 'pptx') {
            $slides = @($archive.Entries | Where-Object { $_.FullName -match '^ppt/slides/slide\d+\.xml$' } | Sort-Object { [int]($_.FullName -replace '\D', '') })
            if ($slides.Count -eq 0 -or $slides.Count -gt 100) { throw 'Import a presentation with between 1 and 100 slides.' }
            $paragraphs = foreach ($slide in $slides) {
                $document = Read-XmlEntry $slide
                $namespaces = New-Object System.Xml.XmlNamespaceManager($document.NameTable)
                $namespaces.AddNamespace('a', 'http://schemas.openxmlformats.org/drawingml/2006/main')
                "Slide $($slide.FullName -replace '\D', '')"
                foreach ($paragraph in $document.SelectNodes('//a:p', $namespaces)) {
                    ($paragraph.SelectNodes('.//a:t', $namespaces) | ForEach-Object { $_.InnerText }) -join ''
                }
            }
            return $paragraphs -join "`n"
        }
        if ($Format -eq 'xlsx') {
            $shared = @()
            $sharedEntry = $archive.GetEntry('xl/sharedStrings.xml')
            if ($null -ne $sharedEntry) {
                $strings = Read-XmlEntry $sharedEntry
                $namespaces = New-Object System.Xml.XmlNamespaceManager($strings.NameTable)
                $namespaces.AddNamespace('s', 'http://schemas.openxmlformats.org/spreadsheetml/2006/main')
                $shared = @($strings.SelectNodes('//s:si', $namespaces) | ForEach-Object {
                    ($_.SelectNodes('.//s:t', $namespaces) | ForEach-Object { $_.InnerText }) -join ''
                })
            }
            $sheets = @($archive.Entries | Where-Object { $_.FullName -match '^xl/worksheets/sheet\d+\.xml$' } | Sort-Object { [int]($_.FullName -replace '\D', '') })
            if ($sheets.Count -eq 0 -or $sheets.Count -gt 40) { throw 'Import a spreadsheet with between 1 and 40 worksheets.' }
            $rows = foreach ($sheet in $sheets) {
                $document = Read-XmlEntry $sheet
                $namespaces = New-Object System.Xml.XmlNamespaceManager($document.NameTable)
                $namespaces.AddNamespace('s', 'http://schemas.openxmlformats.org/spreadsheetml/2006/main')
                "Worksheet $($sheet.FullName -replace '\D', '') (stored values; formulas are not recalculated)"
                foreach ($row in $document.SelectNodes('//s:sheetData/s:row', $namespaces)) {
                    $cells = foreach ($cell in $row.SelectNodes('s:c', $namespaces)) {
                        $value = $cell.SelectSingleNode('s:v', $namespaces)
                        if ($cell.GetAttribute('t') -eq 's') {
                            [int]$index = -1
                            if ($null -eq $value -or -not [int]::TryParse($value.InnerText, [ref]$index) -or $index -lt 0 -or $index -ge $shared.Count) { throw 'The spreadsheet contains an invalid text reference.' }
                            "$($cell.GetAttribute('r')): $($shared[$index])"
                        } elseif ($cell.GetAttribute('t') -eq 'inlineStr') {
                            "$($cell.GetAttribute('r')): $(($cell.SelectNodes('s:is//s:t', $namespaces) | ForEach-Object { $_.InnerText }) -join '')"
                        } elseif ($null -ne $value) {
                            "$($cell.GetAttribute('r')): $($value.InnerText)"
                        } elseif ($null -ne $cell.SelectSingleNode('s:f', $namespaces)) {
                            "$($cell.GetAttribute('r')): [formula result not stored]"
                        }
                    }
                    $cells -join "`t"
                }
            }
            return $rows -join "`n"
        }
    } finally { $archive.Dispose() }
}

function Initialize-Ocr {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
    $null = [Windows.Storage.Streams.IRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]
    $null = [Windows.Storage.Streams.InMemoryRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]
    $null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime]
    $null = [Windows.Graphics.Imaging.SoftwareBitmap, Windows.Graphics.Imaging, ContentType = WindowsRuntime]
    $null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
    $null = [Windows.Media.Ocr.OcrResult, Windows.Foundation, ContentType = WindowsRuntime]
    $null = [Windows.Data.Pdf.PdfDocument, Windows.Data.Pdf, ContentType = WindowsRuntime]
    $script:operationTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
    } | Select-Object -First 1
    $script:actionTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and -not $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction'
    } | Select-Object -First 1
    $script:engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
    if ($null -eq $script:engine) { throw 'Install a Windows OCR language in Settings, then retry, or paste the text.' }
}

function Wait-Operation($Operation, [Type]$ResultType) {
    $task = $script:operationTask.MakeGenericMethod($ResultType).Invoke($null, @($Operation))
    return $task.GetAwaiter().GetResult()
}

function Wait-Action($Operation) {
    $task = $script:actionTask.Invoke($null, @($Operation))
    $task.GetAwaiter().GetResult()
}

function Read-StreamText($Stream) {
    $decoder = Wait-Operation ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($Stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
    if ($decoder.PixelWidth -gt 12000 -or $decoder.PixelHeight -gt 12000 -or ([long]$decoder.PixelWidth * $decoder.PixelHeight) -gt 40000000) {
        throw 'The image exceeds the supported pixel dimensions. Resize it before importing.'
    }
    $maximum = [Math]::Min([Windows.Media.Ocr.OcrEngine]::MaxImageDimension, 2600)
    $ratio = [Math]::Min(1.0, $maximum / [double][Math]::Max($decoder.PixelWidth, $decoder.PixelHeight))
    $transform = New-Object Windows.Graphics.Imaging.BitmapTransform
    $transform.ScaledWidth = [uint32][Math]::Max(1, [Math]::Floor($decoder.PixelWidth * $ratio))
    $transform.ScaledHeight = [uint32][Math]::Max(1, [Math]::Floor($decoder.PixelHeight * $ratio))
    $bitmap = Wait-Operation ($decoder.GetSoftwareBitmapAsync(
        [Windows.Graphics.Imaging.BitmapPixelFormat]::Bgra8,
        [Windows.Graphics.Imaging.BitmapAlphaMode]::Premultiplied,
        $transform,
        [Windows.Graphics.Imaging.ExifOrientationMode]::RespectExifOrientation,
        [Windows.Graphics.Imaging.ColorManagementMode]::DoNotColorManage
    )) ([Windows.Graphics.Imaging.SoftwareBitmap])
    try {
        $recognized = Wait-Operation ($script:engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
        return (($recognized.Lines | ForEach-Object { $_.Text }) -join "`n")
    } finally { $bitmap.Dispose() }
}

function Read-ImageText {
    Initialize-Ocr
    $file = Wait-Operation ([Windows.Storage.StorageFile]::GetFileFromPathAsync($InputFile)) ([Windows.Storage.StorageFile])
    $stream = Wait-Operation ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
    try { return Read-StreamText $stream } finally { $stream.Dispose() }
}

function Read-PdfText {
    Initialize-Ocr
    $file = Wait-Operation ([Windows.Storage.StorageFile]::GetFileFromPathAsync($InputFile)) ([Windows.Storage.StorageFile])
    $pdf = Wait-Operation ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
    if ($pdf.PageCount -gt 12) { throw 'Import a PDF with at most 12 pages, or split the file first.' }
    $pages = foreach ($index in 0..([int]$pdf.PageCount - 1)) {
        $page = $pdf.GetPage([uint32]$index)
        $stream = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
        try {
            $options = New-Object Windows.Data.Pdf.PdfPageRenderOptions
            $scale = 2000.0 / [Math]::Max($page.Size.Width, $page.Size.Height)
            $options.DestinationWidth = [uint32][Math]::Max(1, [Math]::Floor($page.Size.Width * $scale))
            $options.DestinationHeight = [uint32][Math]::Max(1, [Math]::Floor($page.Size.Height * $scale))
            Wait-Action ($page.RenderToStreamAsync($stream, $options))
            $stream.Seek(0)
            "Page $($index + 1)`n$(Read-StreamText $stream)"
        } finally {
            $stream.Dispose()
            $page.Dispose()
        }
    }
    return $pages -join "`n`n"
}

function Read-VideoCaptions {
    $ffmpeg = Get-Command ffmpeg.exe, ffmpeg -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $ffmpeg) { throw 'Video import needs a transcript. Upload an SRT/VTT caption file or paste the transcript; no speech-recognition tool is installed.' }
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $ffmpeg.Source
    # Force a real media container. Never probe uploaded playlists or permit
    # network protocols or QuickTime external data references.
    $demuxer = if ($Format -in @('mp4', 'mov')) { 'mov -enable_drefs 0 -use_absolute_path 0' } else { 'matroska' }
    $start.Arguments = '-nostdin -v error -protocol_whitelist file,pipe -f ' + $demuxer + ' -i "' + $InputFile + '" -map 0:s:0 -f srt -'
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::Start($start)
    try {
        $readCaptions = $process.StandardOutput.ReadToEndAsync()
        $readErrors = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(30000)) {
            $process.Kill()
            $process.WaitForExit()
            throw 'This video has no readable caption track. Upload an SRT/VTT caption file or paste its transcript. Speech and video frames are not interpreted.'
        }
        $captions = $readCaptions.GetAwaiter().GetResult()
        $null = $readErrors.GetAwaiter().GetResult()
        if ($process.ExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($captions)) {
            throw 'This video has no readable caption track. Upload an SRT/VTT caption file or paste its transcript. Speech and video frames are not interpreted.'
        }
        return $captions
    } finally { $process.Dispose() }
}

try {
    $text = switch ($Format) {
        { $_ -in @('docx', 'pptx', 'xlsx') } { Read-OfficeText }
        'pdf' { Read-PdfText }
        { $_ -in @('png', 'jpg', 'jpeg', 'bmp') } { Read-ImageText }
        { $_ -in @('mp4', 'webm', 'mov', 'mkv') } { Read-VideoCaptions }
        default { throw 'This file format is not supported.' }
    }
    if ([string]::IsNullOrWhiteSpace($text)) { throw 'No readable text was found. Try a clearer image or paste the text.' }
    if ($utf8.GetByteCount($text) -gt $maximumText) { throw 'The extracted text exceeds 180 KiB. Split the file first.' }
    [System.IO.File]::WriteAllText($OutputFile, (@{ text = $text } | ConvertTo-Json -Compress), $utf8)
    exit 0
} catch {
    $message = 'The file could not be read. Use a supported, unencrypted file or paste its text.'
    $knownMessages = @(
        'The Word file has too many archive entries.', 'This Word archive contains unsupported entries.',
        'The expanded Word file exceeds 32 MiB.', 'The Word document is missing or too large.',
        'An Office document part is missing or exceeds 2 MiB.',
        'Import a presentation with between 1 and 100 slides.',
        'Import a spreadsheet with between 1 and 40 worksheets.',
        'The spreadsheet contains an invalid text reference.',
        'Video import needs a transcript. Upload an SRT/VTT caption file or paste the transcript; no speech-recognition tool is installed.',
        'This video has no readable caption track. Upload an SRT/VTT caption file or paste its transcript. Speech and video frames are not interpreted.',
        'Install a Windows OCR language in Settings, then retry, or paste the text.',
        'The image exceeds the supported pixel dimensions. Resize it before importing.',
        'Import a PDF with at most 12 pages, or split the file first.',
        'No readable text was found. Try a clearer image or paste the text.',
        'The extracted text exceeds 180 KiB. Split the file first.'
    )
    if ($_.Exception.Message -in $knownMessages) { $message = $_.Exception.Message }
    [System.IO.File]::WriteAllText($OutputFile, (@{ error = $message } | ConvertTo-Json -Compress), $utf8)
    exit 1
}
