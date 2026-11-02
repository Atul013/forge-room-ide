export type TokenBudget = { tokens: number; limit: number; fits: boolean };

export async function countTokenBudget(
	serializedMessage: string,
	limit: number,
	countTokens: (text: string) => PromiseLike<number>
): Promise<TokenBudget> {
	if (!Number.isSafeInteger(limit) || limit <= 0) { throw new Error('The selected model did not provide a valid input token limit.'); }
	const tokens = await countTokens(serializedMessage);
	if (!Number.isSafeInteger(tokens) || tokens < 0) { throw new Error('The selected model returned an invalid token estimate.'); }
	return { tokens, limit, fits: tokens <= limit };
}
