export class CSChatEditReviewLens extends Disposable {
	static selector = 'file';

	constructor(
		@ILanguageFeaturesService private readonly languageFeaturesService: ILanguageFeaturesService,
		@ICSChatEditSessionService private readonly csChatEditSessionService: ICSChatEditSessionService,
	) {
		super();

		this._register(this.languageFeaturesService.codeLensProvider.register({ scheme: CSChatEditReviewLens.selector, hasAccessToAllModels: true }, {
			provideCodeLenses: (model: ITextModel, token: CancellationToken) => {
				const { isEditing, activeEditCodeblockNumber: codeblockIndex, activeEditResponseId: responseId } = this.csChatEditSessionService;
				if (isEditing || codeblockIndex === undefined || codeblockIndex < 0) {
					return;
				}

				const editRanges = this.csChatEditSessionService.getEditRangesInProgress(model.uri);
				if (!editRanges) {
					return;
				}

				if (token.isCancellationRequested) {
					return;
				}

				const lenses = editRanges.map(location => {
					const approveCommand = {
						id: EditConfirmationAction.ID,
						title: 'Approve edits',
						arguments: [{ responseId, codeblockIndex, type: 'approve', uri: model.uri }]
					};
					const rejectCommand = {
						id: EditConfirmationAction.ID,
						title: 'Reject edits',
						arguments: [{ responseId, codeblockIndex, type: 'reject', uri: model.uri }]
					};
					return [
						{
							range: location.range,
							command: approveCommand
						},
						{
							range: location.range,
							command: rejectCommand
						}
					];
				}).flat();

				return <CodeLensList>{
					lenses,
					dispose: () => { }
				};
			}
		}));
	}
}

Registry.as<IWorkbenchContributionsRegistry>(WorkbenchExtensions.Workbench).registerWorkbenchContribution(CSChatEditReviewLens, LifecyclePhase.Eventually);
