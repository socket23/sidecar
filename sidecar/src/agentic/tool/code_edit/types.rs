//! We want to invoke the code edit and rewrite a section of the code which we
//! are insterested in
//! The input here is the file_path and the range to edit and the new_output which
//! we want to generate

use std::sync::Arc;

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

use super::models::broker::CodeEditBroker;

#[derive(Clone, Debug)]
pub struct CodeEdit {
    code_above: Option<String>,
    code_below: Option<String>,
    fs_file_path: String,
    code_to_edit: String,
    extra_context: String,
    language: String,
    model: LLMType,
    instruction: String,
    api_key: LLMProviderAPIKeys,
    provider: LLMProvider,
    is_swe_bench_initial_edit: bool,
    symbol_to_edit: Option<String>,
    is_new_symbol_request: Option<String>,
    root_request_id: String,
    // If this edit is just generating an outline of the changes which need to happen
    // in the symbol and not the complete change which needs to happen
    is_outline_edit: bool,
}

impl CodeEdit {
    pub fn new(
        code_above: Option<String>,
        code_below: Option<String>,
        fs_file_path: String,
        code_to_edit: String,
        extra_context: String,
        language: String,
        instruction: String,
        model: LLMType,
        api_key: LLMProviderAPIKeys,
        provider: LLMProvider,
        is_swe_bench_initial_edit: bool,
        symbol_to_edit: Option<String>,
        is_new_symbol_request: Option<String>,
        root_request_id: String,
        is_outline_edit: bool,
    ) -> Self {
        Self {
            code_above,
            code_below,
            fs_file_path,
            code_to_edit,
            extra_context,
            language,
            model,
            instruction,
            api_key,
            provider,
            is_swe_bench_initial_edit,
            symbol_to_edit,
            is_new_symbol_request,
            root_request_id,
            is_outline_edit,
        }
    }
}

pub struct CodeEditingTool {
    llm_client: Arc<LLMBroker>,
    broker: Arc<CodeEditBroker>,
    editor_config: Option<LLMProperties>,
    fail_over_llm: LLMProperties,
}

impl CodeEditingTool {
    pub fn new(
        llm_client: Arc<LLMBroker>,
        broker: Arc<CodeEditBroker>,
        fail_over_llm: LLMProperties,
    ) -> Self {
        Self {
            llm_client,
            broker,
            editor_config: None,
            fail_over_llm,
        }
    }

    pub fn set_editor_config(mut self, editor_config: Option<LLMProperties>) -> Self {
        self.editor_config = editor_config;
        self
    }

    pub fn get_llm_properties(&self) -> Option<&LLMProperties> {
        self.editor_config.as_ref()
    }

    /// Code output from LLMs is of the following form:
    /// {garbage}
    /// <reply>
    /// <thinking>
    /// thinking inside....
    /// </thinking>
    /// <code_edited>
    /// ```{language}
    /// {content}
    /// ```
    /// </code_edited>
    /// </reply>
    /// {garbage}
    /// So we find this pattern and trim it out if we can
    fn edit_code(
        code: &str,
        new_sub_symbol: bool,
        section_to_edit: &str,
    ) -> Result<String, ToolError> {
        let tag_to_search = if new_sub_symbol {
            "code_to_add"
        } else {
            "code_edited"
        };
        let lines = code
            .lines()
            .skip_while(|line| !line.contains(&format!("<{tag_to_search}>")))
            .skip(1)
            .take_while(|line| !line.contains(&format!("</{tag_to_search}>")))
            .collect::<Vec<_>>()
            .into_iter()
            .skip_while(|line| !line.contains("```"))
            .skip(1)
            .take_while(|line| !line.contains("```"))
            .collect::<Vec<_>>()
            .join("\n");
        if lines == "" {
            Err(ToolError::CodeNotFormatted(code.to_owned()))
        } else {
            if new_sub_symbol {
                Ok(lines + "\n" + section_to_edit + "\n")
            } else {
                Ok(lines)
            }
        }
    }
}

impl CodeEdit {
    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    pub fn above_context(&self) -> Option<&str> {
        self.code_above
            .as_ref()
            .map(|above_context| above_context.as_str())
    }

    pub fn below_context(&self) -> Option<&str> {
        self.code_below
            .as_ref()
            .map(|below_context| below_context.as_str())
    }

    pub fn code_to_edit(&self) -> &str {
        &self.code_to_edit
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn extra_content(&self) -> &str {
        &self.extra_context
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn model(&self) -> &LLMType {
        &self.model
    }

    pub fn is_new_sub_symbol(&self) -> Option<String> {
        self.is_new_symbol_request.clone()
    }

    pub fn symbol_to_edit_name(&self) -> Option<String> {
        self.symbol_to_edit.clone()
    }

    /// Returns if this is an outline edit and not a deep verbose edit which
    /// we want to perform
    pub fn is_outline_edit(&self) -> bool {
        self.is_outline_edit
    }
}

#[async_trait]
impl Tool for CodeEditingTool {
    // TODO(skcd): Figure out how we want to do streaming here in the future
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let code_edit_context = input.is_code_edit()?;
        let root_id = code_edit_context.root_request_id.to_owned();
        let mut llm_message = self.broker.format_prompt(&code_edit_context)?;
        if let Some(llm_properties) = self.get_llm_properties() {
            llm_message = llm_message.set_llm(llm_properties.llm().clone());
        }
        // If this is not special swe bench initial edit then do the overrideas
        // as before
        let (request_llm, request_api_key, request_provider) =
            if !code_edit_context.is_swe_bench_initial_edit {
                if let Some(llm_properties) = self.get_llm_properties() {
                    (
                        llm_properties.llm().clone(),
                        llm_properties.api_key().clone(),
                        llm_properties.provider().clone(),
                    )
                } else {
                    (
                        code_edit_context.model.clone(),
                        code_edit_context.api_key.clone(),
                        code_edit_context.provider.clone(),
                    )
                }
            // if this is the special swe bench initial edit, then keep the llm properties
            // as they are being sent from the invoker
            } else {
                (
                    code_edit_context.model.clone(),
                    code_edit_context.api_key.clone(),
                    code_edit_context.provider.clone(),
                )
            };
        llm_message = llm_message.set_llm(request_llm.clone());
        let mut retries = 0;
        loop {
            if retries >= 4 {
                return Err(ToolError::RetriesExhausted);
            }
            let (llm, api_key, provider) = if retries % 2 == 0 {
                (
                    request_llm.clone(),
                    request_api_key.clone(),
                    request_provider.clone(),
                )
            } else {
                (
                    self.fail_over_llm.llm().clone(),
                    self.fail_over_llm.api_key().clone(),
                    self.fail_over_llm.provider().clone(),
                )
            };
            let cloned_llm_message = llm_message.clone().set_llm(llm);
            let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
            let result = self
                .llm_client
                .stream_completion(
                    api_key,
                    cloned_llm_message,
                    provider,
                    vec![
                        ("event_type".to_owned(), "code_edit_tool".to_owned()),
                        ("root_id".to_owned(), root_id.to_owned()),
                    ]
                    .into_iter()
                    .collect(),
                    sender,
                )
                .await
                .map_err(|e| ToolError::LLMClientError(e))?;
            let edited_code = Self::edit_code(
                &result,
                code_edit_context.is_new_sub_symbol().is_some(),
                code_edit_context.code_to_edit(),
            )
            // we need to do post-processing here to remove all the gunk
            // which usually gets added when we are editing code
            .map(|result| ToolOutput::code_edit_output(result));
            match edited_code {
                Ok(response) => return Ok(response),
                Err(_e) => {
                    retries = retries + 1;
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CodeEditingTool;

    #[test]
    fn test_code_editing() {
        let code = r#"Here is the edited code with the requested change:

<reply>
```python
    def delete(self):
        # sort instance collections
        for model, instances in self.data.items():
            self.data[model] = sorted(instances, key=attrgetter("pk"))

        # if possible, bring the models in an order suitable for databases that
        # don't support transactions or cannot defer constraint checks until the
        # end of a transaction.
        self.sort()
        # number of objects deleted for each model label
        deleted_counter = Counter()

        # Optimize for the case with a single obj and no dependencies
        if len(self.data) == 1 and len(instances) == 1:
            instance = list(instances)[0]
            if self.can_fast_delete(instance):
                with transaction.mark_for_rollback_on_error():
                    count = sql.DeleteQuery(model).delete_batch([instance.pk], self.using)
                return count, {model._meta.label: count}

        with transaction.atomic(using=self.using, savepoint=False):
            # send pre_delete signals
            for model, obj in self.instances_with_model():
                if not model._meta.auto_created:
                    signals.pre_delete.send(
                        sender=model, instance=obj, using=self.using
                    )

            # fast deletes
            for qs in self.fast_deletes:
                count = qs._raw_delete(using=self.using)
                deleted_counter[qs.model._meta.label] += count

            # update fields
            for model, instances_for_fieldvalues in self.field_updates.items():
                for (field, value), instances in instances_for_fieldvalues.items():
                    query = sql.UpdateQuery(model)
                    query.update_batch([obj.pk for obj in instances],
                                        {field.name: value}, self.using)

            # reverse instance collections
            for instances in self.data.values():
                instances.reverse()

            # delete instances
            for model, instances in self.data.items():
                query = sql.DeleteQuery(model)
                pk_list = [obj.pk for obj in instances]
                count = query.delete_batch(pk_list, self.using)
                deleted_counter[model._meta.label] += count

                if not model._meta.auto_created:
                    for obj in instances:
                        signals.post_delete.send(
                            sender=model, instance=obj, using=self.using
                        )

        # update collected instances
        for instances_for_fieldvalues in self.field_updates.values():
            for (field, value), instances in instances_for_fieldvalues.items():
                for obj in instances:
                    setattr(obj, field.attname, value)
        for model, instances in self.data.items():
            for instance in instances:
                setattr(instance, model._meta.pk.attname, None)
        return sum(deleted_counter.values()), dict(deleted_counter)
```
</reply>

The only change made is in the last loop, where we set the primary key attribute of each instance to `None` after deletion:

```python
for model, instances in self.data.items():
    for instance in instances:
        setattr(instance, model._meta.pk.attname, None)
```

This ensures that the primary key of the deleted instances is cleared after the deletion process is complete."#.to_owned();
        let edit_code = CodeEditingTool::edit_code(&code, false, "").expect("to work");
        let better_data = r#"    def delete(self):
        # sort instance collections
        for model, instances in self.data.items():
            self.data[model] = sorted(instances, key=attrgetter(&quot;pk&quot;))

        # if possible, bring the models in an order suitable for databases that
        # don&apos;t support transactions or cannot defer constraint checks until the
        # end of a transaction.
        self.sort()
        # number of objects deleted for each model label
        deleted_counter = Counter()

        # Optimize for the case with a single obj and no dependencies
        if len(self.data) == 1 and len(instances) == 1:
            instance = list(instances)[0]
            if self.can_fast_delete(instance):
                with transaction.mark_for_rollback_on_error():
                    count = sql.DeleteQuery(model).delete_batch([instance.pk], self.using)
                return count, {model._meta.label: count}

        with transaction.atomic(using=self.using, savepoint=False):
            # send pre_delete signals
            for model, obj in self.instances_with_model():
                if not model._meta.auto_created:
                    signals.pre_delete.send(
                        sender=model, instance=obj, using=self.using
                    )

            # fast deletes
            for qs in self.fast_deletes:
                count = qs._raw_delete(using=self.using)
                deleted_counter[qs.model._meta.label] += count

            # update fields
            for model, instances_for_fieldvalues in self.field_updates.items():
                for (field, value), instances in instances_for_fieldvalues.items():
                    query = sql.UpdateQuery(model)
                    query.update_batch([obj.pk for obj in instances],
                                        {field.name: value}, self.using)

            # reverse instance collections
            for instances in self.data.values():
                instances.reverse()

            # delete instances
            for model, instances in self.data.items():
                query = sql.DeleteQuery(model)
                pk_list = [obj.pk for obj in instances]
                count = query.delete_batch(pk_list, self.using)
                deleted_counter[model._meta.label] += count

                if not model._meta.auto_created:
                    for obj in instances:
                        signals.post_delete.send(
                            sender=model, instance=obj, using=self.using
                        )

        # update collected instances
        for instances_for_fieldvalues in self.field_updates.values():
            for (field, value), instances in instances_for_fieldvalues.items():
                for obj in instances:
                    setattr(obj, field.attname, value)
        for model, instances in self.data.items():
            for instance in instances:
                setattr(instance, model._meta.pk.attname, None)
        return sum(deleted_counter.values()), dict(deleted_counter)"#;
        assert_eq!(edit_code, better_data);
    }

    #[test]
    fn parsing_code_edit() {
        let response = r#"
<reply>
<thinking>
The user wants to add comments to the `RequestEvents` enum variants. I will add a comment to each variant explaining its purpose.
</thinking>
<code_edited>
#[derive(Debug, serde::Serialize)]
pub enum RequestEvents {
    /// Indicates the start of a probing interaction.
    ProbingStart,
    /// Signifies the completion of a probe, carrying the probe's response.
    ProbeFinished(RequestEventProbeFinished),
}
</code_edited>
</reply>
        "#
        .to_owned();
        let edit_code = CodeEditingTool::edit_code(&response, false, "").expect("to work");
    }
}
