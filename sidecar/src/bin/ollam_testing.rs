use async_openai::Client;
use async_openai::{config::OpenAIConfig, types::CreateCompletionRequestArgs};
use futures::StreamExt;
use reqwest::Client as ReqwestClient;
use sidecar::reporting::posthog::client::{posthog_client, PosthogClient};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct OllamaResponse {
    model: String,
    response: String,
    done: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let posthog_client = posthog_client("finetune-test");
    let _ = main_func(posthog_client).await;
    Ok(())
}

async fn main_func(posthog_client: PosthogClient) -> anyhow::Result<()> {
    let api_base = "http://localhost:11434/api/generate".to_owned();
    let api_key = "not-set".to_owned();

    let client = ReqwestClient::new();

    let instruction = r#"<s>[INST]### Instruction\nYou are expert at code changes, you are going to pay attention to the code snippet provided to you in In-Range and make changes to it as per the Query.\nYou are provided the file context above and below the code snippet in ### Above and  ### Below section and some extra code references in ### Code References. These provide additional code context from the codebase which you should use to generate the changes and answer the user query.\n\n### Code references\nCode Symbol: ChatEditor class\n```typescript\n// FILEPATH: src/vs/workbench/contrib/chat/browser/chatEditor.ts\n\nexport class ChatEditor extends EditorPane {\n\tprivate widget!: ChatWidget;\n\n\tprivate _scopedContextKeyService!: IScopedContextKeyService;\n\toverride get scopedContextKeyService() {\n\t\treturn this._scopedContextKeyService;\n\t}\n\n\tprivate _memento: Memento | undefined;\n\tprivate _viewState: IChatViewState | undefined;\n\n\tconstructor(\n\t\t@ITelemetryService telemetryService: ITelemetryService,\n\t\t@IThemeService themeService: IThemeService,\n\t\t@IInstantiationService private readonly instantiationService: IInstantiationService,\n\t\t@IStorageService private readonly storageService: IStorageService,\n\t\t@IContextKeyService private readonly contextKeyService: IContextKeyService,\n\t) {\n\t\tsuper(ChatEditorInput.EditorID, telemetryService, themeService, storageService);\n\t}\n\n\tpublic async clear() {\n\t\treturn this.instantiationService.invokeFunction(clearChatEditor);\n\t}\n\n\tprotected override createEditor(parent: HTMLElement): void {\n\t\tthis._scopedContextKeyService = this._register(this.contextKeyService.createScoped(parent));\n\t\tconst scopedInstantiationService = this.instantiationService.createChild(new ServiceCollection([IContextKeyService, this.scopedContextKeyService]));\n\n\t\tthis.widget = this._register(\n\t\t\tscopedInstantiationService.createInstance(\n\t\t\t\tChatWidget,\n\t\t\t\t{ resource: true },\n\t\t\t\t{ supportsFileReferences: true },\n\t\t\t\t{\n\t\t\t\t\tlistForeground: editorForeground,\n\t\t\t\t\tlistBackground: editorBackground,\n\t\t\t\t\tinputEditorBackground: inputBackground,\n\t\t\t\t\tresultEditorBackground: editorBackground\n\t\t\t\t}));\n\t\tthis._register(this.widget.onDidClear(() => this.clear()));\n\t\tthis.widget.render(parent);\n\t\tthis.widget.setVisible(true);\n\t}\n\n\tpublic override focus(): void {\n\t\tsuper.focus();\n\n\t\tthis.widget?.focusInput();\n\t}\n\n\toverride clearInput(): void {\n\t\tthis.saveState();\n\t\tsuper.clearInput();\n\t}\n\n\toverride async setInput(input: ChatEditorInput, options: IChatEditorOptions, context: IEditorOpenContext, token: CancellationToken): Promise<void> {\n\t\tsuper.setInput(input, options, context, token);\n\n\t\tconst editorModel = await input.resolve();\n\t\tif (!editorModel) {\n\t\t\tthrow new Error(`Failed to get model for chat editor. id: ${input.sessionId}`);\n\t\t}\n\n\t\tif (!this.widget) {\n\t\t\tthrow new Error('ChatEditor lifecycle issue: no editor widget');\n\t\t}\n\n\t\tthis.updateModel(editorModel.model);\n\t}\n\n\tprivate updateModel(model: IChatModel): void {\n\t\tthis._memento = new Memento('interactive-session-editor-' + model.sessionId, this.storageService);\n\t\tthis._viewState = this._memento.getMemento(StorageScope.WORKSPACE, StorageTarget.MACHINE) as IChatViewState;\n\t\tthis.widget.setModel(model, { ...this._viewState });\n\t}\n\n\tprotected override saveState(): void {\n\t\tthis.widget?.saveState();\n\n\t\tif (this._memento && this._viewState) {\n\t\t\tconst widgetViewState = this.widget.getViewState();\n\t\t\tthis._viewState!.inputValue = widgetViewState.inputValue;\n\t\t\tthis._memento!.saveMemento();\n\t\t}\n\t}\n\n\toverride layout(dimension: dom.Dimension, position?: dom.IDomPosition | undefined): void {\n\t\tif (this.widget) {\n\t\t\tthis.widget.layout(dimension.height, dimension.width);\n\t\t}\n\t}\n}\n```\n\n### File content\n\n#### Above\n```typescript\n// FILEPATH: src/vs/workbench/contrib/chat/browser/chatEditor.ts\n// BEGIN: abpxx6d04wxr\n/*---------------------------------------------------------------------------------------------\n *  Copyright (c) Microsoft Corporation. All rights reserved.\n *  Licensed under the MIT License. See License.txt in the project root for license information.\n *--------------------------------------------------------------------------------------------*/\n\nimport 'vs/css!./media/chatEditor';\nimport * as dom from 'vs/base/browser/dom';\nimport { CancellationToken } from 'vs/base/common/cancellation';\nimport { IContextKeyService, IScopedContextKeyService } from 'vs/platform/contextkey/common/contextkey';\nimport { IEditorOptions } from 'vs/platform/editor/common/editor';\nimport { IInstantiationService } from 'vs/platform/instantiation/common/instantiation';\nimport { ServiceCollection } from 'vs/platform/instantiation/common/serviceCollection';\nimport { IStorageService, StorageScope, StorageTarget } from 'vs/platform/storage/common/storage';\nimport { ITelemetryService } from 'vs/platform/telemetry/common/telemetry';\nimport { editorBackground, editorForeground, inputBackground } from 'vs/platform/theme/common/colorRegistry';\nimport { IThemeService } from 'vs/platform/theme/common/themeService';\nimport { EditorPane } from 'vs/workbench/browser/parts/editor/editorPane';\nimport { IEditorOpenContext } from 'vs/workbench/common/editor';\nimport { Memento } from 'vs/workbench/common/memento';\nimport { ChatEditorInput } from 'vs/workbench/contrib/chat/browser/chatEditorInput';\nimport { IChatViewState, ChatWidget } from 'vs/workbench/contrib/chat/browser/chatWidget';\nimport { IChatModel, ISerializableChatData } from 'vs/workbench/contrib/chat/common/chatModel';\nimport { clearChatEditor } from 'vs/workbench/contrib/chat/browser/actions/chatClear';\n\nexport interface IChatEditorOptions extends IEditorOptions {\n\ttarget: { sessionId: string } | { providerId: string } | { data: ISerializableChatData };\n}\n\nexport class ChatEditor extends EditorPane {\n\tprivate widget!: ChatWidget;\n\n\tprivate _scopedContextKeyService!: IScopedContextKeyService;\n\toverride get scopedContextKeyService() {\n\t\treturn this._scopedContextKeyService;\n\t}\n\n\tprivate _memento: Memento | undefined;\n\tprivate _viewState: IChatViewState | undefined;\n\n\tconstructor(\n\t\t@ITelemetryService telemetryService: ITelemetryService,\n\t\t@IThemeService themeService: IThemeService,\n\t\t@IInstantiationService private readonly instantiationService: IInstantiationService,\n\t\t@IStorageService private readonly storageService: IStorageService,\n\t\t@IContextKeyService private readonly contextKeyService: IContextKeyService,\n\t) {\n\t\tsuper(ChatEditorInput.EditorID, telemetryService, themeService, storageService);\n\t}\n\n\tpublic async clear() {\n\t\treturn this.instantiationService.invokeFunction(clearChatEditor);\n\t}\n\n\tprotected override createEditor(parent: HTMLElement): void {\n\t\tthis._scopedContextKeyService = this._register(this.contextKeyService.createScoped(parent));\n\t\tconst scopedInstantiationService = this.instantiationService.createChild(new ServiceCollection([IContextKeyService, this.scopedContextKeyService]));\n\n\t\tthis.widget = this._register(\n\t\t\tscopedInstantiationService.createInstance(\n\t\t\t\tChatWidget,\n\t\t\t\t{ resource: true },\n\t\t\t\t{ supportsFileReferences: true },\n\t\t\t\t{\n\t\t\t\t\tlistForeground: editorForeground,\n\t\t\t\t\tlistBackground: editorBackground,\n\t\t\t\t\tinputEditorBackground: inputBackground,\n\t\t\t\t\tresultEditorBackground: editorBackground\n\t\t\t\t}));\n\t\tthis._register(this.widget.onDidClear(() => this.clear()));\n\t\tthis.widget.render(parent);\n\t\tthis.widget.setVisible(true);\n\t}\n\n\tpublic override focus(): void {\n\t\tsuper.focus();\n\n\t\tthis.widget?.focusInput();\n\t}\n\n\toverride clearInput(): void {\n\t\tthis.saveState();\n\t\tsuper.clearInput();\n\t}\n\n\toverride async setInput(input: ChatEditorInput, options: IChatEditorOptions, context: IEditorOpenContext, token: CancellationToken): Promise<void> {\n\t\tsuper.setInput(input, options, context, token);\n\n\t\tconst editorModel = await input.resolve();\n\t\tif (!editorModel) {\n\t\t\tthrow new Error(`Failed to get model for chat editor. id: ${input.sessionId}`);\n\t\t}\n\n\t\tif (!this.widget) {\n\t\t\tthrow new Error('ChatEditor lifecycle issue: no editor widget');\n\t\t}\n\n\t\tthis.updateModel(editorModel.model);\n\t}\n\n// END: abpxx6d04wxr\n```\n\n#### Below\n```typescript\n// FILEPATH: src/vs/workbench/contrib/chat/browser/chatEditor.ts\n// BEGIN: be15d9bcejpp\n\n\tprotected override saveState(): void {\n\t\tthis.widget?.saveState();\n\n\t\tif (this._memento && this._viewState) {\n\t\t\tconst widgetViewState = this.widget.getViewState();\n\t\t\tthis._viewState!.inputValue = widgetViewState.inputValue;\n\t\t\tthis._memento!.saveMemento();\n\t\t}\n\t}\n\n\toverride layout(dimension: dom.Dimension, position?: dom.IDomPosition | undefined): void {\n\t\tif (this.widget) {\n\t\t\tthis.widget.layout(dimension.height, dimension.width);\n\t\t}\n\t}\n}\n// END: be15d9bcejpp\n```\n\n### In-range\n```typescript\n// FILEPATH: src/vs/workbench/contrib/chat/browser/chatEditor.ts\n// BEGIN: ed8c6549bwf9\n\n\tprivate updateModel(model: IChatModel): void {\n\t\tthis._memento = new Memento('interactive-session-editor-' + model.sessionId, this.storageService);\n\t\tthis._viewState = this._memento.getMemento(StorageScope.WORKSPACE, StorageTarget.MACHINE) as IViewState;\n\t\tthis.widget.setModel(model, { ...this._viewState });\n\t}\n// END: ed8c6549bwf9\n```\n\n### Query\nIn @function:updateModel, cast the memento to IChatViewState instead of IViewState.\n\n### Response [/INST]"#;

    let model_name = "codestory-finetune-inline:latest".to_owned();

    let mut request_body = serde_json::json!({
        "model": model_name,
        "prompt": instruction,
        "temperature": 0.7,
        "stream": true,
        "raw": true,
    });

    let mut response = client
        .post(api_base)
        .json(&request_body)
        .send()
        .await
        .unwrap();

    let mut buffered_string = "".to_owned();

    while let Some(chunk) = response.chunk().await? {
        // Process each chunk as it arrives
        let value = serde_json::from_slice::<OllamaResponse>(chunk.to_vec().as_slice())
            .expect("conversion to not fail");
        buffered_string = buffered_string + &value.response;
        println!("{}", buffered_string);
    }

    Ok(())
}
