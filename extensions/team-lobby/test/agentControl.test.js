const test = require('node:test');
const assert = require('node:assert/strict');
const { pauseAgentRequest, discardAgentRequest } = require('../dist/agentControl');

function active() {
	const state = { paused: false };
	let cancellations = 0;
	return { request: { source: { cancel() { cancellations++; } }, session: state, paused: false }, state, cancellations: () => cancellations };
}

test('pausing cancels current generation once and preserves a resumable paused session', () => {
	const value = active();
	assert.equal(pauseAgentRequest(value.request), true);
	assert.equal(value.request.paused, true);
	assert.equal(value.state.paused, true);
	assert.equal(value.cancellations(), 1);
	assert.equal(pauseAgentRequest(value.request), false);
	assert.equal(value.cancellations(), 1);
});

test('discard cancels the active request without marking it resumable', () => {
	const value = active();
	assert.equal(discardAgentRequest(value.request), true);
	assert.equal(value.request.discardOnCompletion, true);
	assert.equal(value.state.paused, false);
	assert.equal(value.cancellations(), 1);
	assert.equal(pauseAgentRequest(value.request), false);
});
