use llm_client::{
    clients::{
        fireworks::FireworksAIClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{FireworksAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    let system_message = r#"You are an expert software eningeer who never writes incorrect code and is tasked with selecting code symbols whose definitions you can use for editing.
The editor has stopped working for you, so we get no help with auto-complete when writing code, hence we want to make sure that we select all the code symbols which are necessary.
As a first step before making changes, you are tasked with collecting all the definitions of the various code symbols whose methods or parameters you will be using when editing the code in the selection.
- You will be given the original user query in <user_query>
- You will be provided the code snippet you will be editing in <code_snippet_to_edit> section.
- The various definitions of the class, method or function (just the high level outline of it) will be given to you as a list in <code_symbol_outline_list>. When writing code you will reuse the methods from here to make the edits, so be very careful when selecting the symbol outlines you are interested in.
- Pay attention to the <code_snippet_to_edit> section and select code symbols accordingly, do not select symbols which we will not be using for making edits.
- Each code_symbol_outline entry is in the following format:
```
<code_symbol>
<name>
{name of the code symbol over here}
</name>
<content>
{the outline content for the code symbol over here}
</content>
</code_symbol>
```
- You have to decide which code symbols you will be using when doing the edits and select those code symbols.
Your reply should be in the following format:
<reply>
<thinking>
</thinking>
<code_symbol_outline_list>
<code_symbol>
<name>
</name>
<file_path>
</file_path>
</code_symbol>
... more code_symbol sections over here as per your requirement
</code_symbol_outline_list>
<reply>

Now we will show you an example of how the output should look like:
<user_query>
We want to implement a new method on symbol event which exposes the initial request question
</user_query>
<code_snippet_to_edit>
```rust
#[derive(Debug, Clone, serde::Serialize)]
pub enum SymbolEvent {
    InitialRequest(InitialRequestData),
    AskQuestion(AskQuestionRequest),
    UserFeedback,
    Delete,
    Edit(SymbolToEditRequest),
    Outline,
    // Probe
    Probe(SymbolToProbeRequest),
}
```
</code_snippet_to_edit>
<code_symbol_outline_list>
<code_symbol>
<name>
InitialRequestData
</name>
<content>
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/initial_request.rs
#[derive(Debug, Clone, serde::Serialize)]
pub struct InitialRequestData {
    original_question: String,
    plan_if_available: Option<String>,
    history: Vec<SymbolRequestHistoryItem>,
    /// We operate on the full symbol instead of the
    full_symbol_request: bool,
}

impl InitialRequestData {
    pub fn new(
        original_question: String,
        plan_if_available: Option<String>,
        history: Vec<SymbolRequestHistoryItem>,
        full_symbol_request: bool,
    ) -> Self
    
    pub fn full_symbol_request(&self) -> bool

    pub fn get_original_question(&self) -> &str

    pub fn get_plan(&self) -> Option<String>

    pub fn history(&self) -> &[SymbolRequestHistoryItem]
}
</content>
</code_symbol>
<code_symbol>
<name>
AskQuestionRequest
</name>
<content>
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/edit.rs
#[derive(Debug, Clone, serde::Serialize)]
pub struct AskQuestionRequest {
    question: String,
}

impl AskQuestionRequest {
    pub fn new(question: String) -> Self

    pub fn get_question(&self) -> &str
}
</content>
</code_symbol>
<code_symbol>
<name>
SymbolToEditRequest
</name>
<content>
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/edit.rs
#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolToEditRequest {
    symbols: Vec<SymbolToEdit>,
    symbol_identifier: SymbolIdentifier,
    history: Vec<SymbolRequestHistoryItem>,
}

impl SymbolToEditRequest {
    pub fn new(
        symbols: Vec<SymbolToEdit>,
        identifier: SymbolIdentifier,
        history: Vec<SymbolRequestHistoryItem>,
    ) -> Self

    pub fn symbols(self) -> Vec<SymbolToEdit>

    pub fn symbol_identifier(&self) -> &SymbolIdentifier

    pub fn history(&self) -> &[SymbolRequestHistoryItem]
}
</content>
</code_symbol>
<code_symbol>
<name>
SymbolToProbeRequest
</name>
<content>
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/probe.rs
#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolToProbeRequest {
    symbol_identifier: SymbolIdentifier,
    probe_request: String,
    original_request: String,
    original_request_id: String,
    history: Vec<SymbolToProbeHistory>,
}

impl SymbolToProbeRequest {
    pub fn new(
        symbol_identifier: SymbolIdentifier,
        probe_request: String,
        original_request: String,
        original_request_id: String,
        history: Vec<SymbolToProbeHistory>,
    ) -> Self

    pub fn symbol_identifier(&self) -> &SymbolIdentifier

    pub fn original_request_id(&self) -> &str

    pub fn original_request(&self) -> &str

    pub fn probe_request(&self) -> &str

    pub fn history_slice(&self) -> &[SymbolToProbeHistory]

    pub fn history(&self) -> String
}
</content>
</code_symbol>
</code_symbol_outline_list>

Your reply should be:
<reply>
<thinking>
The request talks about implementing new methods for the initial request data, so we need to include the initial request data symbol so we edit the code properly.
</thinking>
<code_symbol_outline_list>
<code_symbol>
<name>
InitialRequestData
</name>
<file_path>
/Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/initial_request.rs
</file_path>
</code_symbol>
</code_symbol_outline_list>
</reply>"#;
    let user_message = r#"<user_query>
Original user query:
change this function to not fail: FAIL: test_async_main_with_in_memory_storage (test_main.TestMain.test_async_main_with_in_memory_storage)
Test async_main function with in-memory storage.
----------------------------------------------------------------------
Traceback (most recent call last):
  File "C:\Users\kroen\AppData\Local\Programs\Python\Python312\Lib\unittest\async_case.py", line 90, in _callTestMethod
    if self._callMaybeAsync(method) is not None:
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "C:\Users\kroen\AppData\Local\Programs\Python\Python312\Lib\unittest\async_case.py", line 112, in _callMaybeAsync
    return self._asyncioRunner.run(
           ^^^^^^^^^^^^^^^^^^^^^^^^
  File "C:\Users\kroen\AppData\Local\Programs\Python\Python312\Lib\asyncio\runners.py", line 118, in run
    return self._loop.run_until_complete(task)
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "C:\Users\kroen\AppData\Local\Programs\Python\Python312\Lib\asyncio\base_events.py", line 664, in run_until_complete
    return future.result()
           ^^^^^^^^^^^^^^^
  File "C:\Users\kroen\AppData\Local\Programs\Python\Python312\Lib\unittest\mock.py", line 1404, in patched
    return await func(*newargs, **newkeywargs)
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "D:\codebuddyschedula\tests\test_main.py", line 116, in test_async_main_with_in_memory_storage
    mock_memory.assert_called_once()
  File "C:\Users\kroen\AppData\Local\Programs\Python\Python312\Lib\unittest\mock.py", line 923, in assert_called_once
    raise AssertionError(msg)
AssertionError: Expected 'ConversationBufferMemory' to have been called once. Called 0 times.

Edit selection reason:
The `async_main` function needs to be modified to ensure that `ConversationBufferMemory` is correctly initialized and called when using in-memory storage. This will address the test failure where `ConversationBufferMemory` was expected to be called but wasn&apos;t.
</user_query>

<file_path>
d:\CodebuddySchedula\src\main.py
</file_path>

<code_above>
from datetime import datetime
import warnings
from googleapiclient import discovery
from langchain.schema import AIMessage

from src.constants import TIMEZONE

warnings.filterwarnings("ignore", "file_cache is unavailable when using oauth2client >= 4.0.0")
discovery.DISCOVERY_CACHE_DISABLED = True

import asyncio
import os
from typing import Dict, Any

from langchain.prompts import MessagesPlaceholder, ChatPromptTemplate
from langchain_community.chat_models import ChatAnthropic, ChatOpenAI
from langchain_community.chat_message_histories import RedisChatMessageHistory
from langchain.memory import ConversationBufferMemory
from langchain.agents import AgentExecutor, create_openai_functions_agent
from langchain_community.vectorstores import Redis as RedisVectorStore
from langchain_openai import OpenAIEmbeddings
from src.redis_config import RedisMemory, REDIS_URL

from src.calendar_tools import create_calendar_tools, get_calendar_service
from src.chains import create_chains
from src.cli_interface import cli
from src.logging_config import setup_logging
from src.model_management import get_available_models, initialize_llm, select_model
from src.gamification import GamificationSystem
from src.custom_agent import CustomAgent

try:
    from langchain import hub
except ImportError:
    print("Could not import langchainhub. Please install with 'pip install langchainhub'")
    hub = None

logger = setup_logging(log_level="DEBUG")  # Set to "INFO" in production

from redis.exceptions import RedisError
from src.redis_config import RedisMemory

class AgentManager:
    def __init__(self, llm, tools, session_id):
        self.llm = llm
        self.tools = tools
        self.session_id = session_id
        self._memory = self._setup_memory()
        self.agent = self._create_agent()
        self.executor = AgentExecutor(agent=self.agent, tools=self.tools, memory=self._memory, verbose=True)
        self.gamification = GamificationSystem()

    @property
    def memory(self):
        if self._memory is None:
            logger.warning("Memory is not initialized. Attempting to set up memory.")
            self._memory = self._setup_memory()
        return self._memory

    def _setup_memory(self):
        try:
            memory = RedisMemory(session_id=self.session_id)
            logger.info(f"RedisMemory initialized with session_id: {self.session_id}")
            return memory
        except RedisError as e:
            logger.warning(f"Failed to connect to Redis: {str(e)}. Using in-memory storage instead.")
            memory = ConversationBufferMemory(return_messages=True)
            if not hasattr(memory, 'clear'):
                memory.clear = lambda: memory.chat_memory.clear()
            return memory
        except Exception as e:
            logger.error(f"Unexpected error setting up memory: {str(e)}. Using in-memory storage.")
            memory = ConversationBufferMemory(return_messages=True)
            if not hasattr(memory, 'clear'):
                memory.clear = lambda: memory.chat_memory.clear()
            return memory

    def _create_agent(self):
        logger.debug("Creating agent")
        if hub:
            prompt = hub.pull("wfh/langsmith-agent-prompt:latest")
        else:
            prompt = ChatPromptTemplate.from_messages([
                ("system", "You are a helpful AI assistant."),
                ("human", "{input}"),
            ])
        return create_openai_functions_agent(self.llm, self.tools, prompt)

    async def process_user_input(self, user_input: str) -> str:
        try:
            logger.debug(f"Processing user input: {user_input}")
            
            response = await self.executor.ainvoke({"input": user_input})
            logger.debug(f"Received response: {response}")
            
            points = await asyncio.to_thread(self.gamification.update_points, user_input)
            total_points = await asyncio.to_thread(self.gamification.get_total_points)
            gamification_result = f"\n\nYou earned {points} points! Total: {total_points}"
            
            final_response = f"Agent response: {response['output']}{gamification_result}"
            logger.info(f"Final response prepared: {final_response}")
            return final_response
        except Exception as e:
            logger.exception(f"Error processing user input: {str(e)}")
            return f"An error occurred while processing your request: {str(e)}"

    async def clear_memory(self):
        try:
            if hasattr(self.memory, 'clear'):
                await asyncio.to_thread(self.memory.clear)
                logger.debug("Memory cleared successfully")
            else:
                logger.warning("Memory object does not have a clear method")
        except Exception as e:
            logger.error(f"Error clearing memory: {str(e)}")

    async def check_redis_connection(self):
        if isinstance(self.memory, RedisMemory):
            try:
                await asyncio.to_thread(self.memory.redis.ping)
                logger.info("Redis connection is active")
                return True
            except RedisError as e:
                logger.error(f"Redis connection check failed: {str(e)}")
                return False
            except Exception as e:
                logger.error(f"Unexpected error during Redis connection check: {str(e)}")
                return False
        else:
            logger.info("Not using Redis memory, using in-memory storage")
            return False

import logging

</code_above>
<code_below>

async def main():
    await cli.run(async_main)

if __name__ == "__main__":
    available_models = get_available_models()
    logger.info(f"Available models: {available_models}")
    asyncio.run(main())
</code_below>
<code_in_selection>
async def async_main(user_input: str, llm: Any = None) -> str:
    logging.debug("Entering async_main")
    try:
        # Initialize AgentManager with default values
        agent_manager = AgentManager(None, [], "test_session")
        
        # Check Redis connection
        redis_connected = False
        redis_error_message = ""
        try:
            redis_connected = await asyncio.wait_for(agent_manager.check_redis_connection(), timeout=5.0)
            logging.info(f"Redis connection status: {'Connected' if redis_connected else 'Not connected'}")
        except asyncio.TimeoutError:
            redis_error_message = "Redis connection check timed out"
        except AttributeError:
            redis_error_message = "AgentManager doesn't support Redis checks"
        except (RedisError, ConnectionError) as e:
            redis_error_message = f"Redis connection failed: {str(e)}"
        except Exception as e:
            redis_error_message = f"Unexpected error: {str(e)}"

        if not redis_connected:
            logging.warning(f"Failed to connect to Redis: {redis_error_message}. Using in-memory storage instead.")

        current_date = datetime.now(TIMEZONE).strftime("%Y-%m-%d %H:%M:%S %Z")
        user_input_with_date = f"{user_input} (Current date and time: {current_date})"
        
        # Create tasks for concurrent execution
        calendar_service_task = asyncio.create_task(get_calendar_service())
        llm_task = asyncio.create_task(asyncio.to_thread(initialize_llm, "anthropic", "claude-3-sonnet-20240229") if llm is None else asyncio.sleep(0))

        # Wait for tasks to complete
        calendar_service, llm = await asyncio.gather(calendar_service_task, llm_task)
        
        # Create tools using the obtained calendar_service
        tools = await asyncio.to_thread(create_calendar_tools, calendar_service)
        
        # Update AgentManager with new llm and tools
        agent_manager.llm = llm
        agent_manager.tools = tools

        # Create chains asynchronously
        try:
            memory = agent_manager.memory
            chains = await asyncio.to_thread(create_chains, llm, memory, None, None, tools)
        except AttributeError:
            logging.error("AgentManager does not have a 'memory' attribute")
            memory = None
            chains = await asyncio.to_thread(create_chains, llm, None, None, None, tools)
        
        # Process user input with a timeout
        try:
            if hasattr(agent_manager, 'process_user_input'):
                response = await asyncio.wait_for(agent_manager.process_user_input(user_input_with_date), timeout=60)
            else:
                logging.error("AgentManager does not have a 'process_user_input' method")
                response = "I'm sorry, but there was an issue processing your request. Please try again later."
        except asyncio.TimeoutError:
            logging.warning("Agent response timed out")
            response = "I apologize, but I couldn't process your request in time. Please try again or simplify your query."
        except AttributeError as ae:
            logging.error(f"AttributeError in agent_manager.process_user_input: {str(ae)}")
            response = "I'm sorry, but there was an issue with the agent's configuration. Please try again later."
        except Exception as e:
            logging.error(f"Unexpected error in process_user_input: {str(e)}")
            response = "An unexpected error occurred while processing your request. Please try again later."

        logging.debug(f"Final response: {response}")
        return response
    except Exception as e:
        logging.exception(f"Error in async_main: {str(e)}")
        return f"Error: An unexpected error occurred: {str(e)}"
<?code_in_selection>

<code_symbol_outline_list>

</code_symbol_outline_list>"#;
    let llm_request = LLMClientCompletionRequest::new(
        LLMType::Llama3_1_8bInstruct,
        vec![
            LLMClientMessage::system(system_message.to_owned()),
            LLMClientMessage::user(user_message.to_owned()),
        ],
        0.2,
        None,
    );
    let client = FireworksAIClient::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let start_instant = std::time::Instant::now();
    let response = client
        .stream_completion(
            LLMProviderAPIKeys::FireworksAI(FireworksAPIKey::new(
                "s8Y7yIXdL0lMeHHgvbZXS77oGtBAHAsfsLviL2AKnzuGpg1n".to_owned(),
            )),
            llm_request,
            sender,
        )
        .await;
    println!(
        "response {}:\n{}",
        start_instant.elapsed().as_millis(),
        response.expect("to work always")
    );
}
