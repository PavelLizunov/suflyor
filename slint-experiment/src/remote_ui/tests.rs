#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::generated::{MarkdownBlock, TextAskWindow, TileTone, TileWindow};
use super::*;
use slint::{ModelRc, VecModel};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;

#[test]
fn values_round_trip() {
    let color = slint::Color::from_argb_u8(0x80, 0x11, 0x22, 0x33);
    assert_eq!(color.to_json(), json!("#11223380"));
    assert_eq!(slint::Color::from_json(&json!("#11223380")), Some(color));
    assert_eq!(
        slint::Color::from_json(&json!("#112233")).map(|c| c.alpha()),
        Some(255)
    );
    assert_eq!(i32::from_json(&json!(4.6)), Some(5));
    assert_eq!(TileTone::Cyan.to_json(), json!("cyan"));
    assert_eq!(
        TileTone::from_json(&json!("violet")),
        Some(TileTone::Violet)
    );
    assert_eq!(TileTone::from_json(&json!("nope")), None);
    let block = MarkdownBlock {
        kind: 2,
        text: "x".into(),
        ..Default::default()
    };
    let wire = block.to_json();
    assert_eq!(wire["display-text"], json!(""));
    assert_eq!(MarkdownBlock::from_json(&wire), Some(block));
}

fn next(lines: &mut impl Iterator<Item = std::io::Result<String>>) -> Value {
    serde_json::from_str(&lines.next().expect("a line").expect("read")).expect("json")
}

/// One test owns the process-wide platform: Slint allows setting it once.
#[test]
fn windows_mirror_to_the_shell_socket() {
    let dir = std::env::temp_dir().join(format!("suflyor-remote-ui-{}", std::process::id()));
    let socket = dir.join("shell.sock");
    install(&socket).expect("install");
    let client = UnixStream::connect(&socket).expect("connect");
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let deadline = Instant::now() + Duration::from_secs(5);
    while OUTBOX.lock().unwrap().is_none() {
        assert!(Instant::now() < deadline, "shell connection not registered");
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut lines = BufReader::new(client.try_clone().expect("clone")).lines();

    let ask = TextAskWindow::new().expect("window");
    let opened = next(&mut lines);
    assert_eq!(opened["t"], "open");
    assert_eq!(opened["type"], "TextAskWindow");
    let id = opened["win"].as_str().unwrap().to_owned();

    ask.set_active_profile("SRE".into());
    let prop = next(&mut lines);
    assert_eq!(
        (prop["t"].as_str(), prop["name"].as_str()),
        (Some("prop"), Some("active-profile"))
    );
    assert_eq!(prop["value"], "SRE");

    ask.show().expect("show");
    assert_eq!(next(&mut lines)["t"], "show");
    assert!(ask.window().is_visible());

    // A callback fired in QML reaches the Rust handler with its argument.
    let got = Rc::new(RefCell::new(String::new()));
    let sink = got.clone();
    ask.on_submitted(move |q| *sink.borrow_mut() = q.to_string());
    dispatch(&json!({ "t": "call", "win": id, "name": "submitted", "args": ["why?"], "id": 1 }));
    assert_eq!(*got.borrow(), "why?");
    let ret = next(&mut lines);
    assert_eq!(
        (ret["t"].as_str(), ret["id"].as_i64()),
        (Some("ret"), Some(1))
    );

    // A two-way property typed in QML is what Rust reads back.
    dispatch(&json!({ "t": "set", "win": id, "name": "query", "value": "typed" }));
    assert_eq!(ask.get_query(), "typed");

    // A UI function called from Rust goes to the shell.
    ask.invoke_focus();
    let invoke = next(&mut lines);
    assert_eq!(
        (invoke["t"].as_str(), invoke["name"].as_str()),
        (Some("invoke"), Some("focus"))
    );

    // Models edited in place after set_* are diffed and sent.
    let tile = TileWindow::new().expect("tile");
    let tile_id = next(&mut lines)["win"].as_str().unwrap().to_owned();
    let blocks = Rc::new(VecModel::<MarkdownBlock>::default());
    tile.set_blocks(ModelRc::from(blocks.clone()));
    assert_eq!(next(&mut lines)["value"], json!([]));
    blocks.push(MarkdownBlock {
        text: "answer".into(),
        ..Default::default()
    });
    for object in live_objects() {
        object.poll_models();
    }
    let changed = next(&mut lines);
    assert_eq!(changed["win"].as_str(), Some(tile_id.as_str()));
    assert_eq!(changed["value"][0]["text"], "answer");

    // A reconnecting shell gets the full state back.
    dispatch(&json!({ "t": "hello" }));
    let mut snapshots = Vec::new();
    loop {
        let message = next(&mut lines);
        if message["t"] == "ready" {
            break;
        }
        snapshots.push(message);
    }
    let ask_state = snapshots
        .iter()
        .find(|m| m["win"] == id.as_str())
        .expect("ask snapshot");
    assert_eq!(ask_state["visible"], true);
    assert_eq!(ask_state["props"]["query"], "typed");

    drop(tile);
    let closed = next(&mut lines);
    assert_eq!(
        (closed["t"].as_str(), closed["win"].as_str()),
        (Some("close"), Some(tile_id.as_str()))
    );
    let _ = std::fs::remove_dir_all(dir);
}
