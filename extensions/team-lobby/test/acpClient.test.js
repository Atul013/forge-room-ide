const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { runAcpAgent } = require('../dist/acpClient');

const fixture = path.join(__dirname, 'fixtures', 'fake-acp-agent.js');

test('ACP v1 negotiates a session without advertising file, terminal, or MCP access and rejects tool permission', async () => {
	const cwd = path.resolve(__dirname);
	const result = await runAcpAgent(process.execPath, [fixture, 'normal', '; this is one literal argument'], cwd, 'Explain the selected context.', { cancelled: () => false });
	assert.equal(result.agentName, 'fake-agent');
	assert.equal(result.agentVersion, '1.2.3');
	assert.equal(result.stopReason, 'end_turn');
	assert.match(result.text, new RegExp(`cwd=${cwd.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`));
	assert.match(result.text, /caps=\{\}/);
	assert.match(result.text, /mcp=\[\]/);
	assert.match(result.text, /argv=\["; this is one literal argument"\]/);
	assert.match(result.text, /prompt=Explain the selected context\./);
	assert.match(result.text, /permission=\{"outcome":"selected","optionId":"deny"\}/);
});

test('ACP agent with unsupported protocol version is rejected', async () => {
	await assert.rejects(runAcpAgent(process.execPath, [fixture, 'unsupported'], path.resolve(__dirname), 'Hi', { cancelled: () => false }), /unsupported protocol version 2/);
});

test('ACP agent authentication is selected by the user and terminal-only methods are excluded', async () => {
	let offered;
	const result = await runAcpAgent(process.execPath, [fixture, 'auth'], path.resolve(__dirname), 'Hi', {
		cancelled: () => false,
		selectAuthenticationMethod: async methods => { offered = methods; return 'agent-login'; }
	});
	assert.deepEqual(offered.map(method => method.id), ['agent-login']);
	assert.match(result.text, /auth=agent-login/);
	assert.equal(result.authMethodId, 'agent-login');
});

test('ACP authentication can be canceled without creating a session', async () => {
	await assert.rejects(runAcpAgent(process.execPath, [fixture, 'auth'], path.resolve(__dirname), 'Hi', {
		cancelled: () => false,
		selectAuthenticationMethod: async () => undefined
	}), /authentication was canceled/);
});

test('ACP session history loads only when advertised and replayed text is not mistaken for the new answer', async () => {
	let canResume;
	const result = await runAcpAgent(process.execPath, [fixture, 'resume'], path.resolve(__dirname), 'Continue the work.', {
		cancelled: () => false,
		previousSessionId: 'saved-session',
		selectPreviousSession: async availability => { canResume = availability.canLoad; return 'resume'; }
	});
	assert.equal(canResume, true);
	assert.equal(result.sessionId, 'saved-session');
	assert.equal(result.sessionResumable, true);
	assert.equal(result.resumedSession, true);
	assert.match(result.text, /prompt=Continue the work\./);
	assert.doesNotMatch(result.text, /REPLAYED OLD CONVERSATION/);
});

test('ACP agent without loadSession support starts a new session when selected', async () => {
	let canResume;
	const result = await runAcpAgent(process.execPath, [fixture, 'normal'], path.resolve(__dirname), 'Start fresh.', {
		cancelled: () => false,
		previousSessionId: 'saved-session',
		selectPreviousSession: async availability => { canResume = availability.canLoad; return 'new'; }
	});
	assert.equal(canResume, false);
	assert.equal(result.sessionId, 'fake-session');
	assert.equal(result.sessionResumable, false);
	assert.equal(result.resumedSession, false);
});

test('ACP session is not resumed under a different authentication method', async () => {
	let availability;
	const result = await runAcpAgent(process.execPath, [fixture, 'resume-auth'], path.resolve(__dirname), 'New identity.', {
		cancelled: () => false,
		previousSessionId: 'saved-session',
		previousAuthMethodId: 'other-login',
		selectAuthenticationMethod: async () => 'agent-login',
		selectPreviousSession: async value => { availability = value; return 'new'; }
	});
	assert.deepEqual(availability, { canLoad: false, authMatches: false });
	assert.equal(result.sessionId, 'fake-session');
	assert.equal(result.authMethodId, 'agent-login');
	assert.equal(result.resumedSession, false);
});

test('canceling the saved-session choice does not silently start a new conversation', async () => {
	await assert.rejects(runAcpAgent(process.execPath, [fixture, 'resume'], path.resolve(__dirname), 'Continue.', {
		cancelled: () => false,
		previousSessionId: 'saved-session',
		selectPreviousSession: async () => undefined
	}), /session selection was canceled/);
});

test('ACP cancellation sends session/cancel and surfaces a canceled result', async () => {
	let canceled = false;
	const timer = setTimeout(() => { canceled = true; }, 250);
	try {
		await assert.rejects(runAcpAgent(process.execPath, [fixture, 'cancel'], path.resolve(__dirname), 'Hi', { cancelled: () => canceled }), /canceled/i);
	} finally { clearTimeout(timer); }
});

test('ACP command settings reject control characters', async () => {
	await assert.rejects(runAcpAgent(process.execPath, ['bad\nargument'], path.resolve(__dirname), 'Hi', { cancelled: () => false }), /command or arguments are invalid/);
});
