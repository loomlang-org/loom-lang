use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::Command,
    thread,
    time::Duration,
};

mod common;
use common::success;

#[test]
fn source_http_server_interoperates_with_a_binary_pipelined_peer() {
    let package = "compiler/examples/http_server";
    success(&common::loom(&["check", package]));
    success(
        &common::command(&["test", package])
            .env("LOOM_OPT_LEVEL", "0")
            .output()
            .unwrap(),
    );
    let temporary = tempfile::tempdir().unwrap();
    let executable = common::executable(temporary.path(), "http-server");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package])
                .arg("--output")
                .arg(&executable)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        success(&common::run_tasks(&executable));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let peer = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut stream = BufReader::new(socket);
            stream
                .get_mut()
                .write_all(
                    b"POST http://LOCAL.test:80/echo HTTP/1.1\r\n\
                      Host: untrusted.test\r\nContent-Length: 3\r\n\
                      Expect: 100-continue\r\n\r\n",
                )
                .unwrap();
            let mut interim = String::new();
            stream.read_line(&mut interim).unwrap();
            assert_eq!(interim, "HTTP/1.1 100 Continue\r\n");
            interim.clear();
            stream.read_line(&mut interim).unwrap();
            assert_eq!(interim, "\r\n");
            stream
                .get_mut()
                .write_all(b"\x00\xff\x80HEAD /metadata HTTP/1.1\r\nHost: local.test\r\n\r\n")
                .unwrap();
            for index in 0..2 {
                let mut line = String::new();
                stream.read_line(&mut line).unwrap();
                assert_eq!(line, "HTTP/1.1 200 OK\r\n");
                let mut headers = Vec::new();
                loop {
                    line.clear();
                    assert!(stream.read_line(&mut line).unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                    headers.push(line.clone());
                }
                assert!(headers.contains(&"Content-Length: 3\r\n".into()));
                assert_eq!(
                    headers.contains(&"Connection: close\r\n".into()),
                    index == 1
                );
                if index == 0 {
                    let mut body = [0; 3];
                    stream.read_exact(&mut body).unwrap();
                    assert_eq!(body, [0, 255, 128]);
                }
            }
            let mut suffix = Vec::new();
            stream.read_to_end(&mut suffix).unwrap();
            assert!(suffix.is_empty(), "HEAD sent a body or extra response");
        });
        let output = common::run_task_command(
            Command::new(&executable)
                .arg(address.to_string())
                .env("LOOM_GC_STRESS", "1"),
        );
        peer.join().unwrap();
        success(&output);
        assert_eq!(output.stdout, b"source HTTP server passed\n");
    }
    success(
        &common::command(&["run", package])
            .env("LOOM_OPT_LEVEL", "0")
            .output()
            .unwrap(),
    );
}
