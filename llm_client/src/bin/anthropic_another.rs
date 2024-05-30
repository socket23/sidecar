use llm_client::{
    clients::{
        anthropic::AnthropicClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMClientRole, LLMType},
    },
    provider::{AnthropicAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    let anthropic_api_key = "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".to_owned();
    let anthropic_client = AnthropicClient::new();
    let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));
    let system_prompt = r#"You are a powerful code filtering engine. You have to order the code snippets in the order in which you want to ask them more questions, you will only get to ask these code snippets deeper questions by following various code symbols to their definitions or references.
    - Probing a code snippet implies that you can follow the type of a symbol or function call or declaration if you think we should be following that symbol. 
    - The code snippets will be provided to you in <code_snippet> section which will also have an id in the <id> section.
    - If you want to ask the section with id 0 then you must output in the following format:
    <code_to_probe>
    <id>
    0
    </id>
    <reason_to_probe>
    {your reason for probing}
    </reason_to_probe>
    </code_to_probe>
    - There will be code section which are not necessary to answer the user query, let's say you do not want to ask further questions to the snippet section with id 1, you must provide the reason for not probing and then you must output in the following format:
    <code_to_not_probe>
    <id>
    0
    </id>
    <reason_to_not_probe>
    {your reason for not probing}
    </reason_to_not_probe>
    </code_to_not_probe>
    
    Here is the example contained in the <example> section.
    
    <example>
    <user_query>
    The checkout process is broken. After entering payment info, the order doesn't get created and the user sees an error page.
    </user_query>
    <rerank_list>
    <rerank_entry>
    <id>
    0
    </id>
    <content>
    Code Location: auth.js:5-30
    ```typescript
    const bcrypt = require('bcryptjs');
    const User = require('../models/user');
    router.post('/register', async (req, res) => {{
    const {{ email, password, name }} = req.body;
    try {{
    let user = await User.findOne({{ email }});
    if (user) {{
    return res.status(400).json({{ message: 'User already exists' }});
        }}
    user = new User({{
            email,
            password,
            name
        }});
        const salt = await bcrypt.genSalt(10);
        user.password = await bcrypt.hash(password, salt);
        await user.save();
        req.session.user = user;
    res.json({{ message: 'Registration successful', user }});
    }} catch (err) {{
        console.error(err);
    res.status(500).json({{ message: 'Server error' }});
        }}  
    }});
    
    router.post('/login', async (req, res) => {{
    const {{ email, password }} = req.body;
    
    try {{
    const user = await User.findOne({{ email }});
    if (!user) {{
    return res.status(400).json({{ message: 'Invalid credentials' }});
        }}
    
        const isMatch = await bcrypt.compare(password, user.password);
    if (!isMatch) {{
    return res.status(400).json({{ message: 'Invalid credentials' }});  
        }}
    
        req.session.user = user;
    res.json({{ message: 'Login successful', user }});
    }} catch (err) {{
        console.error(err);
    res.status(500).json({{ message: 'Server error' }});
        }}
    }});
    ```
    </content>
    </rerank_entry>
    <rerank_entry>
    <id>
    1
    </id>
    <content>
    Code Location: cart_model.js:1-20
    ```typescript
    const mongoose = require('mongoose');
    const cartSchema = new mongoose.Schema({{
    user: {{
        type: mongoose.Schema.Types.ObjectId,
        ref: 'User',
        required: true
        }},
    items: [{{
    product: {{
            type: mongoose.Schema.Types.ObjectId,
            ref: 'Product'
        }},
        quantity: Number,
        price: Number  
        }}]
    }}, {{ timestamps: true }});
    cartSchema.virtual('totalPrice').get(function() {{
        return this.items.reduce((total, item) => total + item.price * item.quantity, 0);
    }});
    module.exports = mongoose.model('Cart', cartSchema);
    ```
    </content>
    </rerank_entry>
    <rerank_entry>
    <id>
    2
    </id>
    <content>
    Code Location: order.js:5-25
    ```typescript
    const Order = require('../models/order');
    router.get('/', async (req, res) => {{
    try {{
    const orders = await Order.find({{ user: req.user._id }}).sort('-createdAt');
        res.json(orders);
    }} catch (err) {{
        console.error(err);
    res.status(500).json({{ message: 'Server error' }});
        }}
    }});
    router.get('/:id', async (req, res) => {{
    try {{
    const order = await Order.findOne({{ _id: req.params.id, user: req.user._id }});
    if (!order) {{
    return res.status(404).json({{ message: 'Order not found' }});
        }}
        res.json(order);
    }} catch (err) {{
        console.error(err);
    res.status(500).json({{ message: 'Server error' }});
        }}  
    }});
    </content>
    </rerank_entry>
    <rerank_entry>
    <id>
    3
    </id>
    <content>
    Code Location: checkout.js:5-30
    ```typescript
    router.post('/submit', async (req, res) => {{
    const {{ cartId, paymentInfo }} = req.body;
    try {{
        const cart = await Cart.findById(cartId).populate('items.product');
    if (!cart) {{
    return res.status(404).json({{ message: 'Cart not found' }});
        }}
    const order = new Order({{
            user: req.user._id,
            items: cart.items,
            total: cart.totalPrice,
            paymentInfo,
        }});
        await order.save();
        await Cart.findByIdAndDelete(cartId);
    res.json({{ message: 'Order placed successfully', orderId: order._id }});
    }} catch (err) {{
        console.error(err);
    res.status(500).json({{ message: 'Server error' }});
        }}
    }});
    ```
    </content>
    </rerank_entry>
    <rerank_entry>
    <id>
    4
    </id>
    <content>
    Code Location: user_model.js:1-10
    const mongoose = require('mongoose');
    const userSchema = new mongoose.Schema({{
    email: {{
        type: String,
        required: true,
        unique: true
        }},
    password: {{
        type: String,
        required: true
        }},
        name: String,
        address: String,
        phone: String,
    isAdmin: {{
        type: Boolean,
        default: false  
        }}
    }}, {{ timestamps: true }});
    module.exports = mongoose.model('User', userSchema);
    </content>
    </rerank_entry>
    <rerank_entry>
    <id>
    5
    </id>
    <content>
    Code Location: index.js:10-25
    ```typescript
    const express = require('express');
    const mongoose = require('mongoose');
    const session = require('express-session');
    const MongoStore = require('connect-mongo')(session);
    const app = express();
    mongoose.connect(process.env.MONGO_URI, {{
        useNewUrlParser: true,
        useUnifiedTopology: true
    }});
    app.use(express.json());
    app.use(session({{
        secret: process.env.SESSION_SECRET,
        resave: false,
        saveUninitialized: true,
    store: new MongoStore({{ mongooseConnection: mongoose.connection }})
    }}));
    app.use('/auth', require('./routes/auth'));
    app.use('/cart', require('./routes/cart'));  
    app.use('/checkout', require('./routes/checkout'));
    app.use('/orders', require('./routes/order'));
    app.use('/products', require('./routes/product'));
    app.use((err, req, res, next) => {{
        console.error(err);
    res.status(500).json({{ message: 'Server error' }});
    }});
    const PORT = process.env.PORT || 5000;
    app.listen(PORT, () => console.log(`Server started on port ${{PORT}}`));
    ```
    </content>
    </rerank_entry>
    <rerank_entry>
    <id>
    6
    </id>
    <content>
    Code Loction: payment.js:3-20
    ```typescript
    const stripe = require('stripe')(process.env.STRIPE_SECRET_KEY);
    router.post('/charge', async (req, res) => {{
    const {{ amount, token }} = req.body;
    try {{
    const charge = await stripe.charges.create({{
            amount,
            currency: 'usd',
            source: token,
            description: 'Example charge'
        }});
    res.json({{ message: 'Payment successful', charge }});
    }} catch (err) {{
        console.error(err);  
    res.status(500).json({{ message: 'Payment failed' }});
        }}
    }});
    ```
    </content>
    </rerank_entry>
    <rerank_entry>
    <id>
    7
    </id>
    <content>
    Code Loction: product_model.js:1-12
    ```typescript
    const mongoose = require('mongoose');
    const productSchema = new mongoose.Schema({{
    name: {{
        type: String,
        required: true
        }},
        description: String,
    price: {{
        type: Number,
        required: true,
        min: 0
        }},
    category: {{
        type: String,
        enum: ['electronics', 'clothing', 'home'],
        required: true  
        }},
    stock: {{
        type: Number,
        default: 0,
        min: 0
        }}
    }});
    module.exports = mongoose.model('Product', productSchema);
    ```
    </content>
    </rerank_entry>
    <rerank_entry>
    <id>
    8
    </id>
    <content>
    Code Location: order_model.js:1-15
    ```typescript
    const mongoose = require('mongoose');
    const orderSchema = new mongoose.Schema({{
    user: {{ 
        type: mongoose.Schema.Types.ObjectId,
        ref: 'User',
        required: true
        }},
    items: [{{
    product: {{
            type: mongoose.Schema.Types.ObjectId,
            ref: 'Product'
        }},
        quantity: Number,
        price: Number
        }}],
    total: {{
        type: Number,
        required: true
        }},
    paymentInfo: {{
        type: Object,
        required: true
        }},
    status: {{
        type: String,
        enum: ['pending', 'processing', 'shipped', 'delivered'],
        default: 'pending'
        }}
    }}, {{ timestamps: true }});
    module.exports = mongoose.model('Order', orderSchema);
    ```
    </content>
    </rerank_entry>
    
    <rerank_entry>
    <id>
    9
    </id>
    <content>
    Code Location: cart.js:5-20
    ```typescript
    router.post('/add', async (req, res) => {{
    const {{ productId, quantity }} = req.body;
        
    try {{
    let cart = await Cart.findOne({{ user: req.user._id }});
    if (cart) {{
            const itemIndex = cart.items.findIndex(item => item.product == productId);
    if (itemIndex > -1) {{
            cart.items[itemIndex].quantity += quantity;
    }} else {{
    cart.items.push({{ product: productId, quantity, price: product.price }});
            }}
            cart = await cart.save();
    }} else {{
    cart = await Cart.create({{
            user: req.user._id,
    items: [{{ product: productId, quantity, price: product.price }}]
            }});
        }}
        res.json(cart);
    }} catch (err) {{
        console.error(err);
    res.status(500).json({{ message: 'Server error' }});  
        }}
    }});
    ```
    </content>
    </rerank_entry>
    </rerank_list>
    
    Your reply should be:
    
    <code_to_probe_list>
    <code_to_probe>
    <id>
    3
    </id>
    <reason_to_probe>
    This code handles the checkout process. It receives the cart ID and payment info from the request body. It finds the cart, creates a new order with the cart items and payment info, saves the order, deletes the cart, and returns the order ID. This is likely where the issue is occurring.
    </reason_to_probe>
    <id>
    </code_to_probe>
    <code_to_probe>
    <id>
    6
    </id>
    <reason_to_probe>
    This code processes the actual payment by creating a Stripe charge. The payment info comes from the checkout process. If the payment fails, that could explain the checkout error, so this is important to investigate.
    </reason_to_probe>
    </code_to_probe>
    <code_to_probe>
    <id>
    8
    </id>
    <reason_to_probe>
    This defines the schema and model for orders. An order contains references to the user and product items, the total price, payment info, and status. It's important for understanding the structure of an order, but unlikely to contain bugs.
    </reason_to_probe>
    </code_to_probe>
    </code_to_probe_list>
    <code_to_not_probe_list>
    <code_to_not_probe>
    <id>
    1
    </id>
    <reason_to_not_probe>
    This defines the schema and model for shopping carts. A cart contains references to the user and product items. It also has a virtual property to calculate the total price. It's used in the checkout process but probably not the source of the bug.
    </reason_to_not_probe>
    </code_to_not_probe>
    <code_to_not_probe>
    <id>
    5
    </di>
    <reason_to_not_probe>
    This is the main Express server file. It sets up MongoDB, middleware, routes, and error handling. While it's crucial for the app as a whole, it doesn't contain any checkout-specific logic.
    </reason_to_not_probe>
    </code_to_not_probe>
    <code_to_not_probe>
    <id>
    0
    </id>
    <reason_to_not_probe>
    This code handles user registration and login. It's used to authenticate the user before checkout can occur. But since the error happens after entering payment info, authentication is likely not the problem.
    </reason_to_not_probe>
    </code_to_not_probe>
    <code_to_not_probe>
    <id>
    9
    </id>
    <reason_to_not_probe>
    This code handles adding items to the cart. It's used before the checkout process begins. While it's important for the overall shopping flow, it's unlikely to be directly related to a checkout bug.  
    </reason_to_not_probe>
    </code_to_not_probe>
    <code_to_not_probe>
    <id>
    2
    </id>
    <reason_to_not_probe>
    This code allows fetching the logged-in user's orders. It's used after the checkout process to display order history. It doesn't come into play until after checkout is complete.
    </reason_to_not_probe>
    </code_to_not_probe>
    <code_to_not_probe>
    <id>
    4
    </id>
    <reason_to_not_probe>
    This defines the schema and model for user accounts. A user has an email, password, name, address, phone number, and admin status. The user ID is referenced by the cart and order, but the user model itself is not used in the checkout.
    </reason_to_not_probe>
    </code_to_not_probe>
    <code_to_not_probe>
    <id>
    7
    </id>
    <reason_to_not_probe>
    This defines the schema and model for products. A product has a name, description, price, category, and stock quantity. It's referenced by the cart and order models but is not directly used in the checkout process.
    </reason_to_not_probe>
    </code_to_not_probe>
    </code_to_not_probe_list>
    </example>
    
    Always remember that you have to reply in the following format:
    <code_to_probe_list>
    {list of snippets we want to probe}
    </code_to_probe_list>
    <code_to_not_probe_list>
    {list of snippets we want to not probe anymore}
    </code_to_not_probe_list>
    If there are no snippets which need to be probed then reply with an emply list of items for <code_to_not_probe_list>.
    Similarly if there are no snippets which you need to probe then reply with an emplty list of items for <code_to_probe_list>.
    
    These example is for reference. You must strictly follow the format shown in the example when replying.
    
    Some more examples of outputs and cases you need to handle:
    <example>
    <scenario>
    there are no <code_to_not_probe_list> items
    </scenario>
    <output>
    </code_to_probe_list>
    <code_to_probe>
    <id>
    0
    </id>
    <reason_to_probe>
    {your reason for probing this code section}
    </reason_to_probe>
    <code_to_probe>
    {more code to probe list items...}
    </code_to_probe_list>
    </code_to_not_probe_list>
    </code_to_not_probe_list>
    
    Notice how we include the elements for <code_to_probe_list> and even if the <code_to_not_probe_list> is empty we still output it as empty list.
    </example>
    <example>
    <scenario>
    there are no <code_to_probe_list> items
    </scenario>
    <output>
    <code_to_probe_list>
    </code_to_probe_list>
    <code_to_not_probe_list>
    <code_to_not_probe>
    <id>
    0
    </id>
    <reason_to_not_probe>
    {your reason for not probing}
    </reason_to_not_probe>
    </code_to_not_probe>
    {more code to not probe list items here...}
    </code_to_not_probe_list>
    </output>
    </example>
    <example>
    <scenario>
    Example with both <code_to_probe_list> and <code_to_not_probe_list>
    </scenario>
    <output>
    <code_to_probe_list>
    <code_to_probe>
    <id>
    0
    </id>
    <reason_to_probe>
    {your reason for probing this code section}
    </reason_to_probe>
    <code_to_probe>
    {more code to probe list items...}
    </code_to_probe_list>
    <code_to_not_probe_list>
    <code_to_not_probe>
    <id>
    1
    </id>
    <reason_to_not_probe>
    {your reason for probing this code section}
    </reason_to_not_probe>
    <code_to_not_probe>
    {more code to not probe list items which strictly follow the same format as above}
    </code_to_not_probe_list>
    </output>
    </example>
    
    In this example we still include the <code_to_probe_list> section even if there are no code sections which we need to probe.
    
    Please provide the order along with the reason in 2 lists, one for code snippets which you want to probe and the other for symbols we do not have to probe to answer the user query."#;
    let fim_request = r#"<user_query>
The `Agent::prepare_for_search` function seems to be the entry point for setting up the `Agent` instance for handling a search query. To understand how the LLM client is invoked for the search, we should follow the implementation of the `Agent::answer` method, which appears to be responsible for generating the answer using the LLM broker and other components.
</user_query>

<rerank_list>
<rerank_entry>
<id>
0
</id>
<content>
Code location: /Users/skcd/scratch/sidecar/sidecar/src/agent/search.rs:83-1094
```
impl Agent {
    pub fn prepare_for_search(
        application: Application,
        reporef: RepoRef,
        session_id: uuid::Uuid,
        query: &str,
        llm_broker: Arc<LLMBroker>,
        conversation_id: uuid::Uuid,
        sql_db: SqlDb,
        mut previous_conversations: Vec<ConversationMessage>,
        sender: Sender<ConversationMessage>,
        editor_parsing: EditorParsing,
        model_config: LLMClientConfig,
        llm_tokenizer: Arc<LLMTokenizer>,
        chat_broker: Arc<LLMChatModelBroker>,
        reranker: Arc<ReRankBroker>,
    ) -> Self {
        // We will take care of the search here, and use that for the next steps
        let conversation_message = ConversationMessage::search_message(
            conversation_id,
            AgentState::Search,
            query.to_owned(),
        );
        previous_conversations.push(conversation_message);
        let agent = Agent {
            application,
            reporef,
            session_id,
            conversation_messages: previous_conversations,
            llm_broker,
            sql_db,
            sender,
            user_context: None,
            project_labels: vec![],
            editor_parsing,
            model_config,
            llm_tokenizer,
            chat_broker,
            reranker,
            system_instruction: None,
        };
        agent
    }

    pub fn prepare_for_followup(
        application: Application,
        reporef: RepoRef,
        session_id: uuid::Uuid,
        llm_broker: Arc<LLMBroker>,
        sql_db: SqlDb,
        conversations: Vec<ConversationMessage>,
        sender: Sender<ConversationMessage>,
        user_context: UserContext,
        project_labels: Vec<String>,
        editor_parsing: EditorParsing,
        model_config: LLMClientConfig,
        llm_tokenizer: Arc<LLMTokenizer>,
        chat_broker: Arc<LLMChatModelBroker>,
        reranker: Arc<ReRankBroker>,
        system_instruction: Option<String>,
    ) -> Self {
        let agent = Agent {
            application,
            reporef,
            session_id,
            conversation_messages: conversations,
            llm_broker,
            sql_db,
            sender,
            user_context: Some(user_context),
            project_labels,
            editor_parsing,
            model_config,
            llm_tokenizer,
            chat_broker,
            reranker,
            system_instruction,
        };
        agent
    }

    pub fn prepare_for_semantic_search(
        application: Application,
        reporef: RepoRef,
        session_id: uuid::Uuid,
        query: &str,
        llm_broker: Arc<LLMBroker>,
        conversation_id: uuid::Uuid,
        sql_db: SqlDb,
        mut previous_conversations: Vec<ConversationMessage>,
        sender: Sender<ConversationMessage>,
        editor_parsing: EditorParsing,
        model_config: LLMClientConfig,
        llm_tokenizer: Arc<LLMTokenizer>,
        chat_broker: Arc<LLMChatModelBroker>,
        reranker: Arc<ReRankBroker>,
    ) -> Self {
        let conversation_message = ConversationMessage::semantic_search(
            conversation_id,
            AgentState::SemanticSearch,
            query.to_owned(),
        );
        previous_conversations.push(conversation_message);
        let agent = Agent {
            application,
            reporef,
            session_id,
            conversation_messages: previous_conversations,
            llm_broker,
            sql_db,
            sender,
            user_context: None,
            project_labels: vec![],
            editor_parsing,
            model_config,
            llm_tokenizer,
            chat_broker,
            reranker,
            system_instruction: None,
        };
        agent
    }

    pub async fn path_search(&mut self, query: &str) -> Result<String> {
        // Here we first take the user query and perform a lexical search
        // on all the paths which are present
        let mut path_matches = self
            .application
            .indexes
            .file
            .fuzzy_path_match(self.reporef(), query, PATH_LIMIT_USIZE)
            .await
            .map(|c| c.relative_path)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        // Now we try semantic search on the same query
        if self.application.semantic_client.is_some() && path_matches.is_empty() {
            path_matches = self
                .application
                .semantic_client
                .as_ref()
                .expect("is_some to hold above")
                .search(query, self.reporef(), PATH_LIMIT, 0, 0.0, true)
                .await?
                .into_iter()
                .map(|payload| payload.relative_path)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
        }

        // This also updates the path in the last exchange which has happened
        // with the agent
        let mut paths = path_matches
            .iter()
            .map(|p| (self.get_path_alias(p), p.to_string()))
            .collect::<Vec<_>>();
        paths.sort_by(|a: &(usize, String), b| a.0.cmp(&b.0));

        let response = paths
            .iter()
            .map(|(alias, path)| format!("{}: {}", alias, path))
            .collect::<Vec<_>>()
            .join("\n");

        // Now we want to update the path in agent
        let last_exchange = self.get_last_conversation_message();
        last_exchange.add_agent_step(super::types::AgentStep::Path {
            query: query.to_owned(),
            response: response.to_owned(),
            paths: paths
                .into_iter()
                .map(|path_with_alias| path_with_alias.1)
                .collect(),
        });

        Ok(response)
    }

    pub fn update_user_selected_variables(&mut self, user_variables: Vec<VariableInformation>) {
        let last_exchange = self.get_last_conversation_message();
        user_variables.into_iter().for_each(|user_variable| {
            last_exchange.add_user_variable(user_variable);
        })
    }

    pub fn save_extended_code_selection_variables(
        &mut self,
        extended_variable_information: Vec<ExtendedVariableInformation>,
    ) -> anyhow::Result<()> {
        for variable_information in extended_variable_information.iter() {
            let last_exchange = self.get_last_conversation_message();
            last_exchange.add_extended_variable_information(variable_information.clone());
        }
        Ok(())
    }

    pub fn save_code_snippets_response(
        &mut self,
        query: &str,
        code_snippets: Vec<CodeSpan>,
    ) -> anyhow::Result<String> {
        for code_snippet in code_snippets
            .iter()
            .filter(|code_snippet| !code_snippet.is_empty())
        {
            // Update the last conversation context with the code snippets which
            // we got here
            let last_exchange = self.get_last_conversation_message();
            last_exchange.add_code_spans(code_snippet.clone());
        }

        let response = code_snippets
            .iter()
            .filter(|c| !c.is_empty())
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("\n\n");

        // Now we want to also update the step of the exchange to highlight that
        // we did a search here
        let last_exchange = self.get_last_conversation_message();
        last_exchange.add_agent_step(super::types::AgentStep::Code {
            query: query.to_owned(),
            response: response.to_owned(),
            code_snippets: code_snippets
                .into_iter()
                .filter(|code_snippet| !code_snippet.is_empty())
                .collect(),
        });

        // Now that we have done the code search, we need to figure out what we
        // can do next with all the snippets, some ideas here include dedup and
        // also to join snippets together
        Ok(response)
    }

    pub async fn code_search_hybrid(&mut self, query: &str) -> Result<Vec<CodeSpan>> {
        const CODE_SEARCH_LIMIT: u64 = 10;
        if self.application.semantic_client.is_none() {
            return Err(anyhow::anyhow!("no semantic client defined"));
        }
        let results_semantic = self
            .application
            .semantic_client
            .as_ref()
            .expect("is_none to hold")
            .search(query, self.reporef(), CODE_SEARCH_LIMIT, 0, 0.0, true)
            .await?;
        // let hyde_snippets = self.hyde(query).await?;
        // if !hyde_snippets.is_empty() {
        //     let hyde_snippets = hyde_snippets.first().unwrap();
        //     let hyde_search = self
        //         .application
        //         .semantic_client
        //         .as_ref()
        //         .expect("is_none to hold")
        //         .search(
        //             hyde_snippets,
        //             self.reporef(),
        //             CODE_SEARCH_LIMIT,
        //             0,
        //             0.3,
        //             true,
        //         )
        //         .await?;
        //     results_semantic.extend(hyde_search);
        // }

        // Now we do a lexical search as well this is to help figure out which
        // snippets are relevant
        let lexical_search_code_snippets = self
            .application
            .indexes
            .code_snippet
            .lexical_search(
                self.reporef(),
                query,
                CODE_SEARCH_LIMIT
                    .try_into()
                    .expect("conversion to not fail"),
            )
            .await
            .unwrap_or(vec![]);

        // Now we get the statistics from the git log and use that for scoring
        // as well
        let git_log_score =
            GitLogScore::generate_git_log_score(self.reporef.clone(), self.application.sql.clone())
                .await;

        let mut code_snippets_semantic = results_semantic
            .into_iter()
            .map(|result| {
                let path_alias = self.get_path_alias(&result.relative_path);
                // convert it to a code snippet here
                let code_span = CodeSpan::new(
                    result.relative_path,
                    path_alias,
                    result.start_line,
                    result.end_line,
                    result.text,
                    result.score,
                );
                code_span
            })
            .collect::<Vec<_>>();

        let code_snippets_lexical_score: HashMap<String, (f32, CodeSpan)> =
            lexical_search_code_snippets
                .into_iter()
                .map(|lexical_code_snippet| {
                    let path_alias = self.get_path_alias(&lexical_code_snippet.relative_path);
                    // convert it to a code snippet here
                    let code_span = CodeSpan::new(
                        lexical_code_snippet.relative_path,
                        path_alias,
                        lexical_code_snippet.line_start,
                        lexical_code_snippet.line_end,
                        lexical_code_snippet.content,
                        Some(lexical_code_snippet.score),
                    );
                    (
                        code_span.get_unique_key(),
                        (lexical_code_snippet.score, code_span),
                    )
                })
                .collect();

        // Now that we have the git log score, lets use that to score the results
        // Lets first get the lexical scores for the code snippets which we are getting from the search
        code_snippets_semantic = code_snippets_semantic
            .into_iter()
            .map(|mut code_snippet| {
                let unique_key = code_snippet.get_unique_key();
                // If we don't get anything here we just return 0.3
                let lexical_score = code_snippets_lexical_score
                    .get(&unique_key)
                    .map(|v| &v.0)
                    .unwrap_or(&0.3);
                let git_log_score = git_log_score.get_score_for_file(&code_snippet.file_path);
                if let Some(semantic_score) = code_snippet.score {
                    code_snippet.score = Some(semantic_score + 2.5 * lexical_score + git_log_score);
                } else {
                    code_snippet.score = Some(2.5 * lexical_score + git_log_score);
                }
                code_snippet
            })
            .collect::<Vec<_>>();

        // We should always include the results from the lexical search, since
        // we have hits for the keywords so they are worth a lot of points
        let code_snippet_semantic_keys: HashSet<String> = code_snippets_semantic
            .iter()
            .map(|c| c.get_unique_key())
            .collect();
        // Now check with the lexical set which are not included in the result
        // and add them
        code_snippets_lexical_score
            .into_iter()
            .for_each(|(_, mut code_snippet_with_score)| {
                // if we don't have it, it makes sense to add the results here and give
                // it a semantic score of 0.3 or something (which is our threshold)
                let unique_key_for_code_snippet = code_snippet_with_score.1.get_unique_key();
                if !code_snippet_semantic_keys.contains(&unique_key_for_code_snippet) {
                    let git_log_score =
                        git_log_score.get_score_for_file(&code_snippet_with_score.1.file_path);
                    code_snippet_with_score.1.score =
                        Some(0.3 + git_log_score + 2.5 * code_snippet_with_score.0);
                    code_snippets_semantic.push(code_snippet_with_score.1);
                }
            });
        code_snippets_semantic.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        Ok(code_snippets_semantic
            .into_iter()
            .take(
                (CODE_SEARCH_LIMIT * 2)
                    .try_into()
                    .expect("20u64 to usize should not fail"),
            )
            .collect())
    }

    /// This code search combines semantic + lexical + git log score
    /// to generate the code snippets which are the most relevant
    pub async fn code_search(&mut self, query: &str) -> Result<String> {
        let code_snippets = self.code_search_hybrid(query).await?;
        self.save_code_snippets_response(query, code_snippets)
    }

    pub async fn process_files(&mut self, _query: &str, _path_aliases: &[usize]) -> Result<String> {
        Ok("".to_owned())
        // const MAX_CHUNK_LINE_LENGTH: usize = 20;
        // const CHUNK_MERGE_DISTANCE: usize = 10;
        // const MAX_TOKENS: usize = 15400;

        // let paths = path_aliases
        //     .iter()
        //     .copied()
        //     .map(|i| self.paths().nth(i).ok_or(i).map(str::to_owned))
        //     .collect::<Result<Vec<_>, _>>()
        //     .map_err(|i| anyhow!("invalid path alias {i}"))?;

        // debug!(?query, ?paths, "processing file");

        // // Immutable reborrow of `self`, to copy freely to async closures.
        // let self_ = &*self;
        // let chunks = futures::stream::iter(paths.clone())
        //     .map(|path| async move {
        //         tracing::debug!(?path, "reading file");

        //         let lines = self_
        //             .get_file_content(&path)
        //             .await?
        //             .with_context(|| format!("path does not exist in the index: {path}"))?
        //             .lines()
        //             .enumerate()
        //             .map(|(i, line)| format!("{} {line}", i + 1))
        //             .collect::<Vec<_>>();

        //         let bpe = tiktoken_rs::get_bpe_from_model("gpt-3.5-turbo")?;

        //         let iter =
        //             tokio::task::spawn_blocking(|| trim_lines_by_tokens(lines, bpe, MAX_TOKENS))
        //                 .await
        //                 .context("failed to split by token")?;

        //         Result::<_>::Ok((iter, path.clone()))
        //     })
        //     // Buffer file loading to load multiple paths at once
        //     .buffered(10)
        //     .map(|result| async {
        //         let (lines, path) = result?;

        //         // The unwraps here should never fail, we generated this string above to always
        //         // have the same format.
        //         let start_line = lines[0]
        //             .split_once(' ')
        //             .unwrap()
        //             .0
        //             .parse::<usize>()
        //             .unwrap()
        //             - 1;

        //         // We store the lines separately, so that we can reference them later to trim
        //         // this snippet by line number.
        //         let contents = lines.join("\n");
        //         let prompt = prompts::file_explanation(query, &path, &contents);

        //         let json = self
        //             .get_llm_client()
        //             .response(
        //                 llm_funcs::llm::OpenAIModel::GPT3_5_16k,
        //                 vec![llm_funcs::llm::Message::system(&prompt)],
        //                 None,
        //                 0.0,
        //                 Some(0.2),
        //             )
        //             .await?;

        //         #[derive(
        //             serde::Deserialize,
        //             serde::Serialize,
        //             PartialEq,
        //             Eq,
        //             PartialOrd,
        //             Ord,
        //             Copy,
        //             Clone,
        //             Debug,
        //         )]
        //         struct Range {
        //             start: usize,
        //             end: usize,
        //         }

        //         #[derive(serde::Serialize)]
        //         struct RelevantChunk {
        //             #[serde(flatten)]
        //             range: Range,
        //             code: String,
        //         }

        //         let mut line_ranges: Vec<Range> = serde_json::from_str::<Vec<Range>>(&json)?
        //             .into_iter()
        //             .filter(|r| r.start > 0 && r.end > 0)
        //             .map(|mut r| {
        //                 r.end = r.end.min(r.start + MAX_CHUNK_LINE_LENGTH); // Cap relevant chunk size by line number
        //                 r
        //             })
        //             .map(|r| Range {
        //                 start: r.start - 1,
        //                 end: r.end,
        //             })
        //             .collect();

        //         line_ranges.sort();
        //         line_ranges.dedup();

        //         let relevant_chunks = line_ranges
        //             .into_iter()
        //             .fold(Vec::<Range>::new(), |mut exps, next| {
        //                 if let Some(prev) = exps.last_mut() {
        //                     if prev.end + CHUNK_MERGE_DISTANCE >= next.start {
        //                         prev.end = next.end;
        //                         return exps;
        //                     }
        //                 }

        //                 exps.push(next);
        //                 exps
        //             })
        //             .into_iter()
        //             .filter_map(|range| {
        //                 Some(RelevantChunk {
        //                     range,
        //                     code: lines
        //                         .get(
        //                             range.start.saturating_sub(start_line)
        //                                 ..=range.end.saturating_sub(start_line),
        //                         )?
        //                         .iter()
        //                         .map(|line| line.split_once(' ').unwrap().1)
        //                         .collect::<Vec<_>>()
        //                         .join("\n"),
        //                 })
        //             })
        //             .collect::<Vec<_>>();

        //         Ok::<_, anyhow::Error>((relevant_chunks, path))
        //     });

        // let processed = chunks
        //     .boxed()
        //     .buffered(5)
        //     .filter_map(|res| async { res.ok() })
        //     .collect::<Vec<_>>()
        //     .await;

        // let mut chunks = processed
        //     .into_iter()
        //     .flat_map(|(relevant_chunks, path)| {
        //         let alias = self.get_path_alias(&path);

        //         relevant_chunks.into_iter().map(move |c| {
        //             CodeSpan::new(
        //                 path.clone(),
        //                 alias,
        //                 c.range.start.try_into().unwrap(),
        //                 c.range.end.try_into().unwrap(),
        //                 c.code,
        //                 None,
        //             )
        //         })
        //     })
        //     .collect::<Vec<_>>();

        // chunks.sort_by(|a, b| a.alias.cmp(&b.alias).then(a.start_line.cmp(&b.start_line)));

        // for chunk in chunks.iter().filter(|c| !c.is_empty()) {
        //     let last_conversation_message = self.get_last_conversation_message();
        //     last_conversation_message.add_code_spans(chunk.clone());
        // }

        // let response = chunks
        //     .iter()
        //     .filter(|c| !c.is_empty())
        //     .map(|c| c.to_string())
        //     .collect::<Vec<_>>()
        //     .join("\n\n");

        // let last_exchange = self.get_last_conversation_message();
        // last_exchange.add_agent_step(AgentStep::Proc {
        //     query: query.to_owned(),
        //     paths,
        //     response: response.to_owned(),
        // });

        // Ok(response)
    }

    pub async fn answer(
        &mut self,
        path_aliases: &[usize],
        sender: tokio::sync::mpsc::UnboundedSender<AgentAnswerStreamEvent>,
    ) -> Result<String> {
        if self.user_context.is_some() {
            let message = self
                .utter_history(Some(2))
                .map(|message| message.to_owned())
                .collect::<Vec<_>>();
            let _ = self
                .answer_context_using_user_data(message, sender.clone())
                .await;
        }
        dbg!("sidecar.generating_context.followup_question");
        let context = self.answer_context(path_aliases).await?;
        let system_prompt = match self.get_last_conversation_message_agent_state() {
            &AgentState::Explain => prompts::explain_article_prompt(
                path_aliases.len() != 1,
                &context,
                &self
                    .reporef()
                    .local_path()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
            // If we are in a followup chat, then we should always use the context
            // from the previous conversation and use that to answer the query
            &AgentState::FollowupChat => {
                let answer_prompt = prompts::followup_chat_prompt(
                    &context,
                    &self
                        .reporef()
                        .local_path()
                        .map(|path| path.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    // If we had more than 1 conversation then this gets counted
                    // as a followup
                    self.conversation_messages_len() > 1,
                    self.user_context.is_some(),
                    &self.project_labels,
                    self.system_instruction.as_ref().map(|s| s.as_str()),
                );
                answer_prompt
            }
            _ => prompts::answer_article_prompt(
                path_aliases.len() != 1,
                &context,
                &self
                    .reporef()
                    .local_path()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        };
        let system_message = llm_funcs::llm::Message::system(&system_prompt);

        let answer_model = self.chat_broker.get_answer_model(self.slow_llm_model())?;
        let history = {
            let h = self.utter_history(None).collect::<Vec<_>>();
            let system_headroom = self.llm_tokenizer.count_tokens(
                self.slow_llm_model(),
                LLMTokenizerInput::Messages(vec![LLMClientMessage::system(
                    system_prompt.to_owned(),
                )]),
            )? as i64;
            let headroom = answer_model.answer_tokens + system_headroom;
            trim_utter_history(h, headroom, answer_model, self.llm_tokenizer.clone())?
        };
        dbg!("sidecar.generating_answer.history_complete");
        let messages = Some(system_message)
            .into_iter()
            .chain(history.into_iter())
            .collect::<Vec<_>>();
        let messages_roles = messages
            .iter()
            .map(|message| message.role())
            .collect::<Vec<_>>();
        dbg!("sidecar.generating_answer.messages", &messages_roles);

        let provider_keys = self
            .provider_for_slow_llm()
            .ok_or(anyhow::anyhow!("no provider keys found for slow model"))?;
        let provider_config = self
            .provider_config_for_slow_model()
            .ok_or(anyhow::anyhow!("no provider config found for slow model"))?;

        let request = LLMClientCompletionRequest::new(
            self.slow_llm_model().clone(),
            messages
                .into_iter()
                .map(|message| (&message).try_into())
                .collect::<Vec<_>>()
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            0.1,
            None,
        )
        .set_max_tokens(
            answer_model
                .answer_tokens
                .try_into()
                .expect("i64 is positive"),
        )
        // fixing the message structure here is necessary for anthropic where we are
        // forced to have alternating human and assistant messages.
        .fix_message_structure();
        let fixed_roles = request
            .messages()
            .iter()
            .map(|message| message.role().clone())
            .collect::<Vec<_>>();
        // dbg!("sidecar.generating_ansewr.fixed_roles", &fixed_roles);
        let (answer_sender, answer_receiver) = tokio::sync::mpsc::unbounded_channel();
        let answer_receiver = UnboundedReceiverStream::new(answer_receiver).map(either::Left);
        let llm_broker = self.llm_broker.clone();
        let reply = llm_broker
            .stream_completion(
                provider_keys.clone(),
                request,
                provider_config.clone(),
                vec![("event_type".to_owned(), "followup_question".to_owned())]
                    .into_iter()
                    .collect(),
                answer_sender,
            )
            .into_stream()
            .map(either::Right);

        let merged_stream = futures::stream::select(answer_receiver, reply);
        let mut final_answer = None;
        pin_mut!(merged_stream);
        while let Some(value) = merged_stream.next().await {
            match value {
                either::Left(llm_answer) => {
                    // we need to send the answer via the stream here
                    let _ = sender.send(AgentAnswerStreamEvent::LLMAnswer(llm_answer));
                }
                either::Right(reply) => {
                    final_answer = Some(reply);
                    break;
                }
            }
        }
        match final_answer {
            Some(Ok(reply)) => {
                let last_message = self.get_last_conversation_message();
                last_message.set_answer(reply.to_owned());
                last_message.set_generated_answer_context(context);
                Ok(reply)
            }
            Some(Err(e)) => Err(e.into()),
            None => Err(anyhow::anyhow!("no answer from llm")),
        }
    }

    fn utter_history(
        &self,
        size: Option<usize>,
    ) -> impl Iterator<Item = llm_funcs::llm::Message> + '_ {
        const ANSWER_MAX_HISTORY_SIZE: usize = 10;

        self.conversation_messages
            .iter()
            .rev()
            .take(
                size.map(|size| std::cmp::min(ANSWER_MAX_HISTORY_SIZE, size))
                    .unwrap_or(ANSWER_MAX_HISTORY_SIZE),
            )
            .rev()
            .flat_map(|conversation_message| {
                let query = Some(llm_funcs::llm::Message::PlainText {
                    content: conversation_message.query().to_owned(),
                    role: llm_funcs::llm::Role::User,
                });

                let conclusion = conversation_message.answer().map(|answer| {
                    llm_funcs::llm::Message::PlainText {
                        role: llm_funcs::llm::Role::Assistant,
                        content: answer.answer_up_until_now.to_owned(),
                    }
                });

                query
                    .into_iter()
                    .chain(conclusion.into_iter())
                    .collect::<Vec<_>>()
            })
    }

    fn get_absolute_path(&self, reporef: &RepoRef, path: &str) -> String {
        let repo_location = reporef.local_path();
        match repo_location {
            Some(ref repo_location) => Path::new(&repo_location)
                .join(Path::new(path))
                .to_string_lossy()
                .to_string(),
            None => {
                // We don't have a repo location, so we just use the path
                path.to_string()
            }
        }
    }

    pub async fn followup_chat_context(&mut self) -> Result<Option<String>> {
        if self.conversation_messages.len() > 1 {
            // we want the last to last chat context here
            self.conversation_messages[self.conversation_messages_len() - 2]
                .get_generated_answer_context()
                .map(|context| Some(context.to_owned()))
                .ok_or(anyhow!("no previous chat"))
        } else {
            Ok(None)
        }
    }

    async fn answer_context(&mut self, aliases: &[usize]) -> Result<String> {
        // Here we create the context for the answer, using the aliases and also
        // using the code spans which we have
        // We change the paths here to be absolute so the LLM can stream that
        // properly
        // Here we might be in a weird position that we have to do followup-chats
        // so for that the answer context is totally different and we set it as such
        let mut prompt = "".to_owned();

        let paths = self.paths().collect::<Vec<_>>();
        let mut aliases = aliases
            .iter()
            .copied()
            .filter(|alias| *alias < paths.len())
            .collect::<Vec<_>>();

        aliases.sort();
        aliases.dedup();

        if !aliases.is_empty() {
            prompt += \"\#\#\#\#\# PATHS \#\#\#\#\#\n\";

            for alias in &aliases {
                let path = &paths[*alias];
                // Now we try to get the absolute path here
                let path_for_prompt = self.get_absolute_path(self.reporef(), path);
                prompt += &format!("{path_for_prompt}\\n");
            }
        }

        let code_spans = self.dedup_code_spans(aliases.as_slice()).await?;

        // Sometimes, there are just too many code chunks in the context, and deduplication still
        // doesn't trim enough chunks. So, we enforce a hard limit here that stops adding tokens
        // early if we reach a heuristic limit.
        let slow_model = self.slow_llm_model();
        let answer_model = self.chat_broker.get_answer_model(slow_model)?;
        let prompt_tokens_used: i64 =
            self.llm_tokenizer
                .count_tokens_using_tokenizer(slow_model, &prompt)? as i64;
        let mut remaining_prompt_tokens: i64 = answer_model.total_tokens - prompt_tokens_used;

        // we have to show the selected snippets which the user has selected
        // we have to show the selected snippets to the prompt as well
        let extended_user_selected_context = self.get_extended_user_selection_information();
        if let Some(extended_user_selection_context_slice) = extended_user_selected_context {
            let user_selected_context_header = \"\#\#\#\# USER SELECTED CONTEXT \#\#\#\#\\n";
            let user_selected_context_tokens: i64 = self
                .llm_tokenizer
                .count_tokens_using_tokenizer(slow_model, user_selected_context_header)?
                as i64;
            if user_selected_context_tokens + answer_model.prompt_tokens_limit
                >= remaining_prompt_tokens
            {
                dbg!("skipping_adding_cause_of_context_length_limit");
                info!(\"we can\'t set user selected context because of prompt limit\");
            } else {
                prompt += user_selected_context_header;
                remaining_prompt_tokens -= user_selected_context_tokens;

                for extended_user_selected_context in
                    extended_user_selection_context_slice.iter().rev()
                {
                    let variable_prompt = extended_user_selected_context.to_prompt();
                    let user_variable_tokens = self
                        .llm_tokenizer
                        .count_tokens_using_tokenizer(slow_model, &variable_prompt)?
                        as i64;
                    if user_variable_tokens + answer_model.prompt_tokens_limit
                        > remaining_prompt_tokens
                    {
                        info!("breaking at {} tokens\", remaining_prompt_tokens);
                        break;
                    }
                    prompt += &variable_prompt;
                    remaining_prompt_tokens -= user_variable_tokens;
                }
            }
        }

        // Select as many recent chunks as possible
        let mut recent_chunks = Vec::new();
        for code_span in code_spans.iter().rev() {
            let snippet = code_span
                .data
                .lines()
                .enumerate()
                .map(|(i, line)| format!("{} {line}\n", i + code_span.start_line as usize + 1))
                .collect::<String>();

            let formatted_snippet = format!(
                \"\#\#\# {} \#\#\#\\n{snippet}\\n\\n",
                self.get_absolute_path(self.reporef(), &code_span.file_path)
            );

            let snippet_tokens: i64 = self
                .llm_tokenizer
                .count_tokens_using_tokenizer(slow_model, &formatted_snippet)?
                as i64;

            if snippet_tokens >= remaining_prompt_tokens {
                dbg!("skipping_code_span_addition", snippet_tokens);
                info!(\"breaking at {} tokens\", remaining_prompt_tokens);
                break;
            }

            recent_chunks.push((code_span.clone(), formatted_snippet));

            remaining_prompt_tokens -= snippet_tokens;
            debug!("{}", remaining_prompt_tokens);
        }

        // group recent chunks by path alias
        let mut recent_chunks_by_alias: HashMap<_, _> =
            recent_chunks
                .into_iter()
                .fold(HashMap::new(), |mut map, item| {
                    map.entry(item.0.alias).or_insert_with(Vec::new).push(item);
                    map
                });

        // write the header if we have atleast one chunk
        if !recent_chunks_by_alias.values().all(Vec::is_empty) {
            prompt += "\n##### CODE CHUNKS #####\n\n";
        }

        // sort by alias, then sort by lines
        let mut aliases = recent_chunks_by_alias.keys().copied().collect::<Vec<_>>();
        aliases.sort();

        for alias in aliases {
            let chunks = recent_chunks_by_alias.get_mut(&alias).unwrap();
            chunks.sort_by(|a, b| a.0.start_line.cmp(&b.0.start_line));
            for (_, formatted_snippet) in chunks {
                prompt += formatted_snippet;
            }
        }

        Ok(prompt)
    }

    async fn dedup_code_spans(&mut self, aliases: &[usize]) -> anyhow::Result<Vec<CodeSpan>> {
        /// The ratio of code tokens to context size.
        ///
        /// Making this closure to 1 means that more of the context is taken up by source code.
        const CONTEXT_CODE_RATIO: f32 = 0.5;

        let answer_model = self.chat_broker.get_answer_model(self.slow_llm_model())?;
        let max_tokens = (answer_model.total_tokens as f32 * CONTEXT_CODE_RATIO) as usize;

        // Note: The end line number here is *not* inclusive.
        let mut spans_by_path = HashMap::<_, Vec<_>>::new();
        for code_span in self
            .code_spans()
            .into_iter()
            .filter(|code_span| aliases.contains(&code_span.alias))
        {
            spans_by_path
                .entry(code_span.file_path.clone())
                .or_default()
                .push(code_span.start_line..code_span.end_line);
        }

        // debug!(?spans_by_path, "expanding code spans");

        let self_ = &*self;
        // Map of path -> line list
        let lines_by_file = futures::stream::iter(&mut spans_by_path)
            .then(|(path, spans)| async move {
                spans.sort_by_key(|c| c.start);
                dbg!("path_for_answer", &path);

                let lines = self_
                    .get_file_content(path)
                    .await
                    .unwrap()
                    .unwrap_or_else(|| panic!("path did not exist in the index: {path}"))
                    .split("\n")
                    .map(str::to_owned)
                    .collect::<Vec<_>>();

                (path.clone(), lines)
            })
            .collect::<HashMap<_, _>>()
            .await;

        debug!(
            event_name = "selected_spans",
            spans_by_path = ?spans_by_path,
        );

        Ok(spans_by_path
            .into_iter()
            .flat_map(|(path, spans)| spans.into_iter().map(move |s| (path.clone(), s)))
            .map(|(path, span)| {
                let line_start = span.start as usize;
                let mut line_end = span.end as usize;
                if line_end >= lines_by_file.get(&path).unwrap().len() {
                    warn!(
                        "line end is greater than the number of lines in the file {}",
                        path
                    );
                    line_end = lines_by_file.get(&path).unwrap().len() - 1;
                }
                let snippet = lines_by_file.get(&path).unwrap()[line_start..line_end].join("\n");

                let path_alias = self.get_path_alias(&path);
                CodeSpan::new(path, path_alias, span.start, span.end, snippet, None)
            })
            .collect())
    }
}
```
</content>
</rerank_entry>
</rerank_list>

Remeber that your reply should be strictly in the following format:
<code_to_probe_list>
{list of snippets we want to probe in the format specified}
</code_to_probe_list>
<code_to_not_probe_list>
{list of snippets we want to not probe anymore in the format specified}
</code_to_not_probe_list>

Remember to include both <code_to_probe_list> and <code_to_not_probe_list> sections, and keep the same XML format which we have told you about."#
        .to_owned();
    let request = LLMClientCompletionRequest::new(
        LLMType::ClaudeSonnet,
        vec![
            LLMClientMessage::new(LLMClientRole::System, system_prompt.to_owned()),
            LLMClientMessage::new(LLMClientRole::User, fim_request.to_owned()),
        ],
        0.1,
        None,
    )
    .set_max_tokens(4096);
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = anthropic_client
        .stream_completion(api_key, request, sender)
        .await;
    println!("{:?}", response);
    // let client = Client::new();
    // let url = "https://api.anthropic.com/v1/messages";
    // let api_key = "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";

    // let response = client
    //     .post(url)
    //     .header("x-api-key", api_key)
    //     .header("anthropic-version", "2023-06-01")
    //     .header("content-type", "application/json")
    //     .json(&json!({
    //         "model": "claude-3-opus-20240229",
    //         "max_tokens": 1024,
    //         "messages": [
    //             {
    //                 "role": "user",
    //                 "content": "Repeat the following content 5 times"
    //             }
    //         ],
    //         "stream": true
    //     }))
    //     .send()
    //     .await
    //     .expect("to work");

    // if response.status().is_success() {
    //     let body = response.text().await.expect("to work");
    //     println!("Response Body: {}", body);
    // } else {
    //     println!("Request failed with status: {}", response.status());
    // }
}
