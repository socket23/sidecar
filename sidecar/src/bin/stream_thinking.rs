use async_stream::stream;
use futures::{pin_mut, stream::Stream, StreamExt};
use serde_xml_rs::from_str;
use sidecar::agentic::tool::code_symbol::models::anthropic::StepListItem;
use tokio::time::{sleep, Duration};

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

    let mut result = vec![];

    let mut buffer = String::new();
    let mut thinking_extracted = false;
    let mut step_list_extracted = vec![];
    let mut processed_up_to = 0;

    while let Some(chunk) = stream.next().await {
        println!("Received chunk: {}", chunk);
        buffer.push_str(&chunk);

        // Attempt to extract the thinking tag's content
        if !thinking_extracted {
            if let Some(content) = extract_tag_content(&buffer, "thinking") {
                println!("Extracted thinking content: {}", content);
                result.push(content);
                thinking_extracted = true;
            }
        }

        // Extract step_list items
        let (step_lists, new_processed_up_to) =
            extract_all_tag_contents(&buffer, "step_list", processed_up_to);
        if !step_lists.is_empty() {
            for step_list in step_lists {
                println!("Extracted step_list content:\n{}", step_list);
                // You can parse the step_list further here
                step_list_extracted.push(step_list);
            }
        }
        processed_up_to = new_processed_up_to;
    }

    // Now, step_list_extracted contains all the extracted <step_list> contents
    println!("All extracted step_list items:");
    for step in step_list_extracted {
        println!("{}", step);
        let wrapped_step = wrap_xml("step_list", &step);
        let output = from_str::<StepListItem>(&wrapped_step);

        dbg!(&output);
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
            sleep(Duration::from_millis(50)).await;
        }
    }
}

fn extract_tag_content(buffer: &str, tag_name: &str) -> Option<String> {
    let tag_start = format!("<{}>", tag_name);
    let tag_end = format!("</{}>", tag_name);
    // Find the start tag
    if let Some(start_index) = buffer.find(&tag_start) {
        // Find the end tag starting from the end of the start tag
        if let Some(end_index) = buffer[start_index..].find(&tag_end) {
            // Extract content between the tags
            let content_start = start_index + tag_start.len();
            let content_end = start_index + end_index;
            let content = &buffer[content_start..content_end];
            return Some(content.to_string());
        }
    }
    None
}

// Function to extract all complete occurrences of a tag's content from the buffer
fn extract_all_tag_contents(
    buffer: &str,
    tag_name: &str,
    start_pos: usize,
) -> (Vec<String>, usize) {
    let tag_start = format!("<{}>", tag_name);
    let tag_end = format!("</{}>", tag_name);
    let mut contents = Vec::new();
    let mut pos = start_pos;
    let buffer_len = buffer.len();

    while pos < buffer_len {
        if let Some(start_index) = buffer[pos..].find(&tag_start) {
            let start_index = pos + start_index;
            if let Some(end_index) = buffer[start_index..].find(&tag_end) {
                let content_start = start_index + tag_start.len();
                let content_end = start_index + end_index;
                let content = &buffer[content_start..content_end];
                contents.push(content.to_string());
                pos = content_end + tag_end.len();
            } else {
                // End tag not found, need more data
                break;
            }
        } else {
            // No more start tags found
            break;
        }
    }

    // Return the accumulated contents and new position
    (contents, pos)
}

fn wrap_xml(root_tag: &str, raw_xml: &str) -> String {
    format!("<{root_tag}>{raw_xml}</{root_tag}>")
}
