"""Append review-only probes to a disposable app copy; never edit native sources."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
APP = ROOT / 'native/forge-room-app'
scratch = Path(tempfile.mkdtemp(prefix='forge-t13-source-'))
copy = scratch / 'app'
copy.mkdir()
for name in ('src', 'assets', 'themes'):
    shutil.copytree(APP / name, copy / name)
for name in ('Cargo.toml', 'Cargo.lock', 'build.rs'):
    shutil.copy2(APP / name, copy / name)
manifest = (copy / 'Cargo.toml').read_text()
manifest = manifest.replace('path = "../forge-room-core"',
                            'path = "' + (ROOT / 'native/forge-room-core').as_posix() + '"')
manifest = manifest.replace('name = "forge-room-early-preview"', 'name = "forge-room-t13-review-probe"')
(copy / 'Cargo.toml').write_text(manifest)

probes = {}
probes['source_control.rs'] = r'''
#[cfg(test)]
mod t13_review {
    use super::*;
    #[test]
    fn t13_narrow_and_many_lanes() {
        for lanes in [1, 16, 40, 48, 64] {
            let ctx = egui::Context::default();
            let commit = git::CommitInfo {hash:"h".into(), parents:vec![], author:"A".into(),
                email:"a@x".into(), time:0, refs:vec![], subject:"T13 subject".into()};
            let row = graph::Row {lane:lanes-1, width:lanes, ..Default::default()};
            let output = ctx.run(egui::RawInput {screen_rect:Some(Rect::from_min_size(
                egui::Pos2::ZERO, egui::vec2(180.0,500.0))), ..Default::default()}, |ctx| {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    graph_row(ui, &commit, &row, lanes, None);
                });
            });
            for shape in output.shapes {
                if let egui::Shape::Text(t) = shape.shape {
                    if t.galley.text() == "T13 subject" {
                        println!("T13_GRAPH lanes={lanes} text_x={} clip={:?} clip_width={}",
                            t.pos.x, shape.clip_rect, shape.clip_rect.width());
                    }
                }
            }
        }
    }
    #[test]
    fn t13_tabs_at_180() {
        let ctx=egui::Context::default();
        let mut state=State::default();
        let out=ctx.run(egui::RawInput {screen_rect:Some(Rect::from_min_size(
            egui::Pos2::ZERO,egui::vec2(180.0,500.0))),..Default::default()},|ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui| {
                page_tabs(ui,&mut state,123,80);
                println!("T13_TABS min_rect={:?}",ui.min_rect());
            });
        });
        for s in out.shapes { if let egui::Shape::Text(t)=s.shape {
            println!("T13_TAB_TEXT {} x={} width={} clip={:?}",t.galley.text(),t.pos.x,
                t.galley.size().x,s.clip_rect);
        }}
    }
}
'''
probes['source_control.rs'] += r'''
#[cfg(test)]
mod t13_row_review {
    use super::*;
    #[test]
    fn t13_two_line_change_at_180() {
        let ctx=egui::Context::default();
        let change=git::Change {path:"src/very/long/directory/long_filename.rs".into(),index:'.',worktree:'M'};
        let mut out=ctx.run_ui(egui::RawInput {screen_rect:Some(Rect::from_min_size(
            egui::Pos2::ZERO,egui::vec2(180.0,500.0))),..Default::default()},|ui| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui,|ui| {
                change_row(ui,&change,false);
            });
        });
        out.textures_delta.clear();
        for s in out.shapes {if let egui::Shape::Text(t)=s.shape {
            println!("T13_CHANGE_TEXT {} x={} y={} clip={:?}",t.galley.text(),t.pos.x,t.pos.y,s.clip_rect);
        }}
    }
}
'''
probes['lobby.rs'] = r'''
#[cfg(test)]
mod t13_review {
    use super::*;
    #[test]
    fn t13_failed_switch_drops_pending_config() {
        let base=std::env::temp_dir().join(format!("t13-lobby-{}",std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let blocker=base.join("blocker");
        std::fs::write(&blocker,"file, not directory").unwrap();
        let mut lobby=Lobby::default();
        lobby.path=Some(blocker.join("lobby.json"));
        lobby.config.goal="ONLY UNSAVED COPY".into(); lobby.dirty=true;
        let result=lobby.load_for(Some(base.join("second.json")));
        println!("T13_SWITCH result={result:?} goal={:?} dirty={} old_saved={}",
            lobby.config.goal,lobby.dirty,blocker.join("lobby.json").exists());
        assert!(result.is_err()); assert_ne!(lobby.config.goal,"ONLY UNSAVED COPY");
        assert!(!lobby.dirty);
    }
    #[test]
    fn t13_small_lobby_header() {
        let ctx=egui::Context::default(); let mut lobby=Lobby::default();
        let facts=Context {workspace:"sample",branch:None,runtimes:&[],state:None,
            room_unavailable:false,acting_as:""};
        let out=ctx.run(egui::RawInput {screen_rect:Some(Rect::from_min_size(
            egui::Pos2::ZERO,vec2(604.0,400.0))),..Default::default()},|ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui| {
                lobby.header(ui,&facts,&mut None);
                println!("T13_HEADER min_rect={:?}",ui.min_rect());
            });
        });
        for s in out.shapes { if let egui::Shape::Text(t)=s.shape {
            println!("T13_HEADER_TEXT {} x={} width={}",t.galley.text(),t.pos.x,t.galley.size().x);
        }}
    }
    #[test]
    fn t13_repository_scheme() {
        let url="httpx://example.test/payload";
        let full=if url.starts_with("http") {url.to_string()} else {format!("https://{url}")};
        println!("T13_BROWSER {full}"); assert!(full.starts_with("httpx://"));
    }
}
'''
probes['syntax.rs'] = r'''
#[cfg(test)]
mod t13_review {
    use super::*;
    #[test]
    fn t13_exact_limit_newline_loses_comment_state() {
        let font=egui::FontId::monospace(14.0);
        let short="/*x\nlet x = 1;\n*/\n";
        let long=format!("/*{}\nlet x = 1;\n*/\n","a".repeat(3998));
        let complete=|text:&str| {
            let mut job=highlight(text,"rs",&font);
            for _ in 0..100 {if !pending() {break;} job=highlight(text,"rs",&font);}
            job
        };
        let color=|job:&LayoutJob| {
            let at=job.text.find("let").unwrap();
            job.sections.iter().find(|s|s.byte_range.start.0<=at && at<s.byte_range.end.0).unwrap().format.color
        };
        let expected=color(&complete(short)); let actual=color(&complete(&long));
        println!("T13_SYNTAX line_without_newline=4000 expected={expected:?} actual={actual:?}");
        assert_ne!(actual,expected);
    }
}
'''
probes['preview.rs'] = r'''
#[cfg(test)]
mod t13_review {
    use super::*;
    #[test]
    fn t13_ipv6_and_schemes() {
        for url in ["http://[::1]/", "http://[::1]:5173/", "http://localhost:5173/",
                    "https://example.com/", "file:///C:/x", "httpx://example.test"] {
            println!("T13_URL {url} parsed={:?} web={} local={}",host_port(url),is_web_url(url),
                host_port(url).is_some_and(|(h,_)| is_local(&h)));
        }
        assert!(!is_web_url("http://[::1]/"));
    }
}
'''
for filename, extra in probes.items():
    extra = extra.replace('ctx.run(', 'ctx.run_ui(').replace('|ctx| {', '|ui| {').replace('.show(ctx,', '.show(ui,')
    extra = extra.replace('let output =', 'let mut output =').replace('let out=', 'let mut out=')
    extra = extra.replace('for shape in output.shapes', 'output.textures_delta.clear(); for shape in output.shapes')
    extra = extra.replace('for s in out.shapes', 'out.textures_delta.clear(); for s in out.shapes')
    with (copy / 'src' / filename).open('a') as f:
        f.write(extra)
env = os.environ.copy()
env['CARGO_TARGET_DIR'] = str(APP / 'target')
cmd = ['cargo', 'test', '--locked', '--offline', '--manifest-path', str(copy/'Cargo.toml'),
       't13_', '--', '--nocapture', '--test-threads=1']
result = subprocess.run(cmd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
out = ROOT / 'design-assets/forge-room-ui-spec/measurements/t13-source-probes.txt'
out.write_text('Command: '+repr(cmd)+'\nDisposable copy: '+str(copy)+'\n'+result.stdout)
print(result.stdout)
print('probe_exit_code=', result.returncode)
