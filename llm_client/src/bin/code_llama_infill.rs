use llm_client::{
    clients::{
        togetherai::TogetherAIClient,
        types::{LLMClient, LLMClientCompletionStringRequest, LLMType},
    },
    provider::{LLMProviderAPIKeys, TogetherAIProvider},
};

#[tokio::main]
async fn main() {
    let api_key = LLMProviderAPIKeys::TogetherAI(TogetherAIProvider {
        api_key: "cc10d6774e67efef2004b85efdb81a3c9ba0b7682cc33d59c30834183502208d".to_owned(),
    });
    let togetherai = TogetherAIClient::new();
    let prompt = r#"<PRE> 		const url = baseUrl.toString();
    const activeWindowData = getCurrentActiveWindow();
    const sideCarModelConfiguration = await getSideCarModelConfiguration(await vscode.modelSelection.getConfiguration());
    const body = {
        repo_ref: repoRef.getRepresentation(),
        query: query,
        thread_id: threadId,
        user_context: await convertVSCodeVariableToSidecar(variables),
        project_labels: projectLabels,
        active_window_data: activeWindowData,
        openai_key: this._openAIKey,
        model_config: sideCarModelConfiguration,
        user_id: this._userId,
    };
    const asyncIterableResponse = await callServerEventStreamingBufferedPOST(url, body);
    for await (const line of asyncIterableResponse) {
        const lineParts = line.split('data:{');
        for (const lineSinglePart of lineParts) {
            const lineSinglePartTrimmed = lineSinglePart.trim();
            if (lineSinglePartTrimmed === '') {
                continue;
            }
            console.log(lineSinglePartTrimmed);
            const conversationMessage = JSON.parse('{' + lineSinglePartTrimmed) as ConversationMessage;
            yield conversationMessage;
        }
    }
}

async *explainQuery(
    query: string,
    repoRef: RepoRef,
    selection: SelectionDataForExplain,
    threadId: string,
): AsyncIterableIterator<ConversationMessage> {
    const baseUrl = new URL(this._url);
    baseUrl.pathname = '/api/agent/explain';
    baseUrl.searchParams.set('repo_ref', repoRef.getRepresentation());
    baseUrl.searchParams.set('query', query);
    baseUrl.searchParams.set('start_line', selection.lineStart.toString());
    baseUrl.searchParams.set('end_line', selection.lineEnd.toString());
    baseUrl.searchParams.set('relative_path', selection.relativeFilePath);
    baseUrl.searchParams.set('thread_id', threadId);
    if (this._openAIKey !== null) {
        baseUrl.searchParams.set('openai_key', this._openAIKey);
    }
    const url = baseUrl.toString();
    const asyncIterableResponse = await callServerEventStreamingBufferedGET(url);
    for await (const line of asyncIterableResponse) {
        const lineParts = line.split('data:{');
        for (const lineSinglePart of lineParts) {
            const lineSinglePartTrimmed = lineSinglePart.trim();
            if (lineSinglePartTrimmed === '') {
                continue;
            }
            const conversationMessage = JSON.parse('{' + lineSinglePartTrimmed) as ConversationMessage;
            yield conversationMessage;
        }
    }
}

async *searchQuery(
    query: string,
    repoRef: RepoRef,
    threadId: string,
): AsyncIterableIterator<ConversationMessage> {
    // how do we create the url properly here?
    const baseUrl = new URL(this._url);
    baseUrl.pathname = '/api/agent/search_agent';
    baseUrl.searchParams.set('reporef', repoRef.getRepresentation());
    baseUrl.searchParams.set('query', query);
    baseUrl.searchParams.set('thread_id', threadId);
    const url = baseUrl.toString();
    const asyncIterableResponse = await callServerEventStreamingBufferedGET(url);
    for await (const line of asyncIterableResponse) {
        // Now these responses can be parsed properly, since we are using our
        // own reader over sse, sometimes the reader might send multiple events
        // in a single line so we should split the lines by \n to get the
        // individual lines
        // console.log(line);
        // Is this a good placeholder? probably not, cause we can have instances
        // of this inside the string too, but for now lets check if this works as
        // want it to
        const lineParts = line.split('data:{');
        for (const lineSinglePart of lineParts) {
            const lineSinglePartTrimmed = lineSinglePart.trim();
            if (lineSinglePartTrimmed === '') {
                continue;
            }
            const conversationMessage = JSON.parse('{' + lineSinglePartTrimmed) as ConversationMessage;
            console.log('[search][stream] whats the message from the stream');
            yield conversationMessage;
        }
    }
}

async inlineCompletion(
    completionRequest: CompletionRequest,
    signal: AbortSignal,
): Promise<CompletionResponse> {
    const baseUrl = new URL(this._url);
    console.log("are we over here in inline completions");
    const sideCarModelConfiguration = await getSideCarModelConfiguration(await vscode.modelSelection.getConfiguration());
    baseUrl.pathname = '/api/inline_completion/inline_completion';

    const body = {
        filepath: completionRequest.filepath,
        language: completionRequest.language,
        text: completionRequest.text,
        // The cursor position in the editor
        position: {
            line: completionRequest.position.line,
            character: completionRequest.position.character,
            byteOffset: completionRequest.position.byteOffset,
        },
        model_config: sideCarModelConfiguration,
    };
    console.log("json string message");
    console.log("" + JSON.stringify(body));
    console.log(body);
    // ssssssssss
    const url = baseUrl.toString();
    console.log(url);

    // Create an instance of AbortController
    const controller = new AbortController();
    const { signal: abortSignal } = controller;

    // Combine the provided signal with the abortSignal
    // const combinedSignal = AbortSignal.abort([signal, abortSignal]);

    // log the body here <SUF>		let response = await fetch(url, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(body),
        // signal: combinedSignal, // Use the combined signal
    });

    // response = await fetch(url, {
    // 	method: 'POST',
    // 	headers: {
    // 		'Content-Type': 'application/json',
    // 	},
    // 	body: JSON.stringify(body),
    // });

    // Check if the request was aborted
    //
    if (signal.aborted) {
        // Send termination notification to the server
        await fetch(url, {
            method: 'DELETE',
        });
        return {
            completions: [],
        }
    }

    const responseJson = await response.json();
    console.log(responseJson);
    return responseJson;
}


async indexRepositoryIfNotInvoked(repoRef: RepoRef): Promise<boolean> {
    // First get the list of indexed repositories
    await this.waitForGreenHC();
    console.log('fetching the status of the various repositories');
    const response = await fetch(this.getRepoListUrl());
    const repoList = (await response.json()) as RepoStatus;
    if (sidecarNotIndexRepository()) {
        return true;
    } <MID>"#;
    let request = LLMClientCompletionStringRequest::new(
        LLMType::CodeLlama13BInstruct,
        prompt.to_owned(),
        0.2,
        None,
    );
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = togetherai
        .stream_prompt_completion(api_key, request, sender)
        .await;
    println!("{}", response.expect("to work"));
}
