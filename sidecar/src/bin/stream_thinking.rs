use async_stream::stream;
use futures::{pin_mut, stream::Stream};
use tokio::time::{sleep, Duration};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let input = r#"<reply>
<thinking>
We need to add a new endpoint for code_request_stop, similar to probe_request_stop, in the agentic router.
</thinking>
<step_by_step>
<step_list>
<name>
agentic_router
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/bin/webserver.rs
</file_path>
<step>
Add a new route for code_request_stop in the agentic_router function
</step>
</step_list>
<step_list>
<name>
code_request_stop
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/agentic.rs
</file_path>
<new>true</new>
<step>
Implement the code_request_stop function, reusing logic from probe_request_stop
</step>
</step_list>
<step_list>
<name>
CodeRequestStop
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/agentic.rs
</file_path>
<new>true</new>
<step>
Create a new struct CodeRequestStop similar to ProbeStopRequest
</step>
</step_list>
<step_list>
<name>
CodeRequestStopResponse
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/agentic.rs
</file_path>
<new>true</new>
<step>
Create a new struct CodeRequestStopResponse similar to ProbeStopResponse
</step>
</step_list>
</step_by_step>
</reply>
"#;

    let chunk_size = 10;
    let stream = simulate_stream(input.to_owned(), chunk_size);

    pin_mut!(stream);

    while let Some(chunk) = stream.next().await {
        println!("Received chunk: {}", chunk);
    }
}

fn simulate_stream(input: String, chunk_size: usize) -> impl Stream<Item = String> {
    stream! {
        let mut index = 0;
        let len = input.len();
        while index < len {
            let end = (index + chunk_size).min(len);
            let chunk = &input[index..end];
            yield chunk.to_string();
            index = end;
            sleep(Duration::from_millis(200)).await;
        }
    }
}
