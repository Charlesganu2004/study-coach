import { writeFile, mkdir } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';

export async function addRecordingCaptions(page) {
  await page.evaluate(() => {
    const host = document.createElement('div');
    document.body.append(host);
    const shadow = host.attachShadow({mode:'open'});
    const style = new CSSStyleSheet();
    style.replaceSync(`
      :host { all:initial; position:fixed; inset:auto 0 0 0; z-index:2147483647; pointer-events:none; }
      section { color:#edf6f2; background:linear-gradient(0deg,#0b181e 0%,#0b181ef7 85%,transparent); padding:1.6vh 4vw 2vh; font:400 .87vw/1.42 "Segoe UI",sans-serif; }
      small { color:#abdccd; font-size:.72em; letter-spacing:.08em; }
      strong { display:block; font-size:1.28em; margin:.2em 0 .1em; }
      p { margin:0; max-width:100ch; }
      .track { position:fixed; left:0; bottom:0; width:100vw; height:10px; background:#182529; }
      .progress { display:block; height:100%; width:0; background:#99dec6; }
    `);
    shadow.adoptedStyleSheets = [style];
    const section = document.createElement('section');
    const label = document.createElement('small');
    label.textContent = 'LOCAL DEMONSTRATION MODEL';
    const title = document.createElement('strong'), body = document.createElement('p');
    section.append(label,title,body);
    const track = document.createElement('div'), progress = document.createElement('span');
    track.className = 'track'; progress.className = 'progress'; track.append(progress);
    shadow.append(section,track);
    window.walkthroughCaption = (heading, text, number, count) => {
      title.textContent=heading; body.textContent=text; progress.style.width=(100*number/count)+'%';
    };
  });
}

// One native screenshot per sample. The final frame stays visible long enough
// to read the caption. FFmpeg preserves those variable presentation times.
export async function recordChapter(page, folder, frames, chapter, seconds, action, quick) {
  await mkdir(folder,{recursive:true});
  const folderFrames = [];
  const sample = async () => {
    const file = 'chapter-' + chapter + '-' + folderFrames.length + '.jpg';
    const pixels = await page.screenshot({type:'jpeg', quality:94, timeout:45000});
    await writeFile(path.join(folder,file),pixels);
    folderFrames.push(file);
  };
  if (action) await action();
  const count = quick ? 2 : chapter === 1 || chapter === 24 ? 22 : 5;
  for (let n=0; n<count; n++) {
    if (n) await page.waitForTimeout(70);
    await sample();
  }
  for (const [n,file] of folderFrames.entries()) {
    const duration = n===count-1 ? seconds - (count-1)/12 : 1/12;
    frames.push({file, duration});
  }
}

export async function encodeChapters(folder, frames, outputFile) {
  const lines=['ffconcat version 1.0'];
  for (const frame of frames) lines.push('file '+frame.file, 'duration '+frame.duration.toFixed(6));
  lines.push('file '+frames.at(-1).file);
  const list=path.join(folder,'frames.ffconcat');
  await writeFile(list,lines.join('\n')+'\n');
  const encoded=spawnSync('ffmpeg',['-hide_banner','-loglevel','error','-nostdin','-y','-f','concat','-safe','1','-i',list,
    '-fps_mode','vfr','-c:v','libvpx-vp9','-deadline','realtime','-cpu-used','8','-row-mt','1','-threads','4',
    '-b:v','0','-crf','28','-pix_fmt','yuv420p',outputFile],{windowsHide:true,stdio:'inherit'});
  if(encoded.error) throw encoded.error;
  if(encoded.status!==0) throw new Error('UHD WebM encoding failed.');
}
