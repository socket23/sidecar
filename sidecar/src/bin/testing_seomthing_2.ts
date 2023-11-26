// Define an asynchronous function named 'sendRequest'
async function sendRequest(accessor: ServicesAccessor, query: string) {
    // Get the chat service from the accessor
    const chatService = accessor.get(ICSChatService);
    // Get the widget service from the accessor
    const widgetService = accessor.get(ICSChatWidgetService);

    // Get the provider ID from the chat service
    const providerId = chatService.getProviderInfos()[0]?.id;
    // Reveal the view for the provider and wait for the widget
    const widget = await widgetService.revealViewForProvider(providerId);
    // If there is no widget, return from the function
    if (!widget) {
        return;
    }

    // Accept the input query in the widget
    widget.acceptInput(query);
}