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
    tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput},
};

use super::models::broker::CodeEditBroker;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
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
        }
    }
}

pub struct CodeEditingTool {
    llm_client: Arc<LLMBroker>,
    broker: Arc<CodeEditBroker>,
    editor_config: Option<LLMProperties>,
}

impl CodeEditingTool {
    pub fn new(llm_client: Arc<LLMBroker>, broker: Arc<CodeEditBroker>) -> Self {
        Self {
            llm_client,
            broker,
            editor_config: None,
        }
    }

    pub fn set_editor_config(mut self, editor_config: Option<LLMProperties>) -> Self {
        self.editor_config = editor_config;
        self
    }

    pub fn get_llm_properties(&self) -> Option<&LLMProperties> {
        self.editor_config.as_ref()
    }

    fn escape_str(line: String) -> String {
        quick_xml::escape::escape(&line).to_string()
    }

    /// Code output from LLMs is of the following form:
    /// {garbage}
    /// <reply>
    /// ```{language}
    /// {content}
    /// ```
    /// </reply>
    /// {garbage}
    /// So we find this pattern and trim it out if we can
    fn edit_code(code: &str) -> Result<String, ToolError> {
        let lines = code
            .lines()
            .skip_while(|line| !line.contains("<reply>"))
            .skip(1)
            .take_while(|line| !line.contains("</reply>"))
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
            Ok(lines)
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
}

#[async_trait]
impl Tool for CodeEditingTool {
    // TODO(skcd): Figure out how we want to do streaming here in the future
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let code_edit_context = input.is_code_edit()?;
        let mut llm_message = self.broker.format_prompt(&code_edit_context)?;
        if let Some(llm_properties) = self.get_llm_properties() {
            llm_message = llm_message.set_llm(llm_properties.llm().clone());
        }
        let (api_key, provider) = if let Some(llm_properties) = self.get_llm_properties() {
            (
                llm_properties.api_key().clone(),
                llm_properties.provider().clone(),
            )
        } else {
            (
                code_edit_context.api_key.clone(),
                code_edit_context.provider.clone(),
            )
        };
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let result = self
            .llm_client
            .stream_completion(
                api_key,
                llm_message,
                provider,
                vec![("request".to_owned(), "code_edit_tool".to_owned())]
                    .into_iter()
                    .collect(),
                sender,
            )
            .await
            .map_err(|e| ToolError::LLMClientError(e))?;
        let edited_code = Self::edit_code(&result)
            // we need to do post-processing here to remove all the gunk
            // which usually gets added when we are editing code
            .map(|result| ToolOutput::code_edit_output(result))?;
        Ok(edited_code)
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
        let edit_code = CodeEditingTool::edit_code(&code).expect("to work");
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
}
