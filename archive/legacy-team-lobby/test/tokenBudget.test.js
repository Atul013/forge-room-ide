const test = require('node:test');
const assert = require('node:assert/strict');
const { countTokenBudget } = require('../dist/tokenBudget');

test('counts the exact serialized message and accepts an estimate at the model limit', async () => {
	const serialized = JSON.stringify(['system prompt', 'selected file contents']);
	let counted;
	const budget = await countTokenBudget(serialized, 20_000, async text => { counted = text; return 20_000; });
	assert.equal(counted, serialized);
	assert.deepEqual(budget, { tokens: 20_000, limit: 20_000, fits: true });
});

test('rejects an over-limit estimate and invalid provider results', async () => {
	assert.deepEqual(await countTokenBudget('request', 10, async () => 11), { tokens: 11, limit: 10, fits: false });
	for (const tokens of [-1, 1.5, Number.NaN, Number.POSITIVE_INFINITY]) {
		await assert.rejects(countTokenBudget('request', 10, async () => tokens), /invalid token estimate/);
	}
	await assert.rejects(countTokenBudget('request', 0, async () => 0), /valid input token limit/);
});
