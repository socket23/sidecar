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
    let prompt = "<PRE> function add(a: number, b: number <SUF>) <MID>".to_owned();
    // let prompt = "<PRE> \t// command we have to run is the following:\n\t// https://chat.openai.com/share/d516b75e-1567-4ce2-b96f-80ba6272adf0\n\tconst stdout = await execCommand(\n\t\t'git log --pretty=\"%H\" --since=\"2 weeks ago\" | while read commit_hash; do git diff-tree --no-commit-id --name-only -r $commit_hash; done | sort | uniq -c | awk -v prefix=\"$(git rev-parse --show-toplevel)/\" \\'{ print prefix $2, $1 }\\' | sort -k2 -rn',\n\t\tworkingDirectory,\n\t);\n\t// Now we want to parse this output out, its always in the form of\n\t// {file_path} {num_tries} and the file path here is relative to the working\n\t// directory\n\tconst splitLines = stdout.split('\\n');\n\tconst finalFileList: string[] = [];\n\tfor (let index = 0; index < splitLines.length; index++) {\n\t\tconst lineInfo = splitLines[index].trim();\n\t\tif (lineInfo.length === 0) {\n\t\t\tcontinue;\n\t\t}\n\t\t// split it by the space\n\t\tconst splitLineInfo = lineInfo.split(' ');\n\t\tif (splitLineInfo.length !== 2) {\n\t\t\tcontinue;\n\t\t}\n\t\tconst filePath = splitLineInfo[0];\n\t\tfinalFileList.push(filePath);\n\t}\n\treturn finalFileList;\n};\n\nfunction add(a <SUF>) {\n// Example usage:\n// (async () => {\n//     const remoteUrl = await getGitRemoteUrl();\n//     console.log(remoteUrl);\n//     const repoHash = await getGitCurrentHash();\n//     console.log(repoHash);\n//     const repoName = await getGitRepoName();\n//     console.log(repoName);\n// })(); <MID>".to_owned();
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
