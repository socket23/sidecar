class CSChatEditReviewLens extends Disposable {
    private _onDidChangeCodeLenses = this._register(new Emitter<void>());
    public readonly onDidChangeCodeLenses = this._onDidChangeCodeLenses.event;

    // rest of your code...

    provideCodeLenses: (model: ITextModel, token: CancellationToken) => {
        const { isEditing, activeEditCodeblockNumber: codeblockIndex, activeEditResponseId: responseId } = this.csChatEditSessionService;
        if (isEditing || codeblockIndex === undefined || codeblockIndex < 0) {
            this._onDidChangeCodeLenses.fire();
            return { lenses: [], dispose: () => {} };
        }

        const editRanges = this.csChatEditSessionService.getEditRangesInProgress(model.uri);
        if (!editRanges) {
            this._onDidChangeCodeLenses.fire();
            return { lenses: [], dispose: () => {} };
        }

        if (token.isCancellationRequested) {
            this._onDidChangeCodeLenses.fire();
            return { lenses: [], dispose: () => {} };
        }

        // rest of code...
    }
}
