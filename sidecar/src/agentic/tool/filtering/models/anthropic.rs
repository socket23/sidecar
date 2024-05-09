use async_trait::async_trait;
use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::agentic::{
    symbol::identifier::Snippet,
    tool::filtering::{
        broker::{CodeToEditFilterFormatter, CodeToEditFilterRequest},
        errors::CodeToEditFilteringError,
    },
};

pub struct AnthropicCodeToEditFormatter {
    llm_broker: Arc<LLMBroker>,
}

impl AnthropicCodeToEditFormatter {
    pub fn new(llm_broker: Arc<LLMBroker>) -> Self {
        Self { llm_broker }
    }

    fn example_message(&self) -> String {
        r#"<example>
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

<code_to_edit_list>
<code_to_edit>
<id>
3
</id>
<reason_to_edit>
This code handles the checkout process. It receives the cart ID and payment info from the request body. It finds the cart, creates a new order with the cart items and payment info, saves the order, deletes the cart, and returns the order ID. This is likely where the issue is occurring.
</reason_to_edit>
<id>
</code_to_edit>
<code_to_edit>
<id>
6
</id>
<reason_to_edit>
This code processes the actual payment by creating a Stripe charge. The payment info comes from the checkout process. If the payment fails, that could explain the checkout error, so this is important to investigate.
</reason_to_edit>
</code_to_edit>
<id>
8
</id>
<reason_to_edit>
This defines the schema and model for orders. An order contains references to the user and product items, the total price, payment info, and status. It's important for understanding the structure of an order, but unlikely to contain bugs.
</reason_to_edit>
<code_to_edit>
</code_to_edit_list>
<code_to_not_edit_list>
<code_to_not_edit>
<id>
1
</id>
<reason_to_not_edit>
This defines the schema and model for shopping carts. A cart contains references to the user and product items. It also has a virtual property to calculate the total price. It's used in the checkout process but probably not the source of the bug.
</reason_to_not_edit>
</code_to_not_edit>
<id>
5
</di>
<reason_to_not_edit>
This is the main Express server file. It sets up MongoDB, middleware, routes, and error handling. While it's crucial for the app as a whole, it doesn't contain any checkout-specific logic.
<<reason_to_not_edit>
<code_to_not_edit>
<id>
0
</id>
<reason_to_not_edit>
This code handles user registration and login. It's used to authenticate the user before checkout can occur. But since the error happens after entering payment info, authentication is likely not the problem.
</reason_to_not_edit>
</code_to_not_edit>
<code_to_not_edit>
<id>
9
</id>
<reason_to_not_edit>
This code handles adding items to the cart. It's used before the checkout process begins. While it's important for the overall shopping flow, it's unlikely to be directly related to a checkout bug.  
</reason_to_not_edit>
</code_to_not_edit>
<code_to_not_edit>
<id>
2
</id>
<reason_to_not_edit>
This code allows fetching the logged-in user's orders. It's used after the checkout process to display order history. It doesn't come into play until after checkout is complete.
</reason_to_not_edit>
</code_to_not_edit>
<code_to_not_edit>
<id>
4
</id>
<reason_to_not_edit>
This defines the schema and model for user accounts. A user has an email, password, name, address, phone number, and admin status. The user ID is referenced by the cart and order, but the user model itself is not used in the checkout.
</reason_to_not_edit>
</code_to_not_edit>
<code_to_not_edit>
<id>
7
</id>
<reason_to_not_edit>
This defines the schema and model for products. A product has a name, description, price, category, and stock quantity. It's referenced by the cart and order models but is not directly used in the checkout process.
</reason_to_not_edit>
</code_to_not_edit>
</code_to_not_edit_list>
</example>"#.to_owned()
    }

    fn system_message(&self) -> String {
        let example_message = self.example_message();
        format!(r#"You are a powerful code filtering engine. You must order the code snippets in the order in you want to edit them, and only those code snippets which should be edited.
- The code snippets will be provided to you in <code_snippet> section which will also have an id in the <id> section.
- If you want to edit the code section with id 0 then you must use output in the following format:
<code_to_edit>
<id>
0
</id>
<edit_reason>
{{your reason for editing}}
</edit_reason>
- There will be code sections which you do not want to edit, let's say you do not want to edit section with id 1, you must provide the reason for not editing and then you must output in the following format:
<code_to_not_edit>
<id>
0
</id>
<no_edit_reason>
{{your reason for not editing}}
</no_edit_reason>
</code_to_not_edit>

Here is an example contained in the <example> section.

{example_message}

This example is for reference. You must strictly follow the format show in the example when replying.
Please provide the order along with the reason in 2 lists, one for the symbols which we should edit and the other for symbols we do not have to edit for the code snippets based on the user's query."#).to_owned()
    }

    fn format_snippet(&self, idx: usize, snippet: &Snippet) -> String {
        let code_location = snippet.file_path();
        let range = snippet.range();
        let start_line = range.start_line();
        let end_line = range.end_line();
        let content = snippet.content();
        let language = snippet.language();
        format!(
            r#"<rerank_entry>
<id>
{idx}
</id>
<content>
Code location: {code_location}:{start_line}-{end_line}
```{language}
{content}
```
</content>
</rerank_entry>"#
        )
        .to_owned()
    }
}

#[async_trait]
impl CodeToEditFilterFormatter for AnthropicCodeToEditFormatter {
    async fn filter_code_snippets(
        &self,
        request: CodeToEditFilterRequest,
    ) -> Result<Vec<Snippet>, CodeToEditFilteringError> {
        // okay now we have the request, send it to the moon and figure out what to
        // do next with it
        let query = request.query();
        let input_list_for_entries = request
            .get_snippets()
            .into_iter()
            .enumerate()
            .map(|(idx, input)| self.format_snippet(idx, input))
            .collect::<Vec<_>>();
        let input_formatted = input_list_for_entries.join("\n");
        let user_query = format!(
            r#"<user_query>
{query}
</user_query>

<rerank_list>
{input_formatted}
</rerank_list>"#
        );
        let system_message = self.system_message();
        let messages = vec![
            LLMClientMessage::system(system_message),
            LLMClientMessage::user(user_query),
        ];
        let llm_request =
            LLMClientCompletionRequest::new(request.llm().clone(), messages, 0.1, None);
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let response = self
            .llm_broker
            .stream_completion(
                request.api_key().clone(),
                llm_request,
                request.provider().clone(),
                vec![("event_type".to_owned(), "code_snippets_to_edit".to_owned())]
                    .into_iter()
                    .collect(),
                sender,
            )
            .await
            .map_err(|e| CodeToEditFilteringError::LLMClientError(e))?;

        // Now to parse that output and reply back to the asking person
        // TODO(skcd):
        // we need to figure out how to parse the output back, it should be easy
        // as its well formatted xml
        // and then we need to change the return types here from raw snippets
        // to snippets with reason to edit and not to edit
        todo!("pick this up from here");
    }
}
