let input = '';
let initializeParams;
let sessionParams;
let sessionId;
let waitingPermission;
let authMethod;

function send(message) { process.stdout.write(`${JSON.stringify(message)}\n`); }
function result(id, value) { send({ jsonrpc: '2.0', id, result: value }); }

process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => {
	input += chunk;
	let newline;
	while ((newline = input.indexOf('\n')) >= 0) {
		const line = input.slice(0, newline).trim();
		input = input.slice(newline + 1);
		if (!line) { continue; }
		const message = JSON.parse(line);
		if (message.method === 'initialize') {
			initializeParams = message.params;
			const authMethods = ['auth', 'resume-auth'].includes(process.argv[2]) ? [{ id: 'agent-login', name: 'Sign in', description: 'Authenticate with the agent account.' }, { id: 'terminal-login', name: 'Terminal login', type: 'terminal' }] : [];
			result(message.id, { protocolVersion: process.argv[2] === 'unsupported' ? 2 : 1, agentInfo: { name: 'fake-agent', version: '1.2.3' }, agentCapabilities: { ...(['resume', 'resume-auth'].includes(process.argv[2]) ? { loadSession: true } : {}), sessionCapabilities: {} }, authMethods });
		} else if (message.method === 'authenticate') {
			authMethod = message.params.methodId;
			result(message.id, {});
		} else if (message.method === 'session/new') {
			sessionParams = message.params;
			sessionId = 'fake-session';
			result(message.id, { sessionId });
		} else if (message.method === 'session/load') {
			sessionParams = message.params;
			sessionId = message.params.sessionId;
			send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId, update: { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: 'REPLAYED OLD CONVERSATION' } } } });
			result(message.id, {});
		} else if (message.method === 'session/prompt') {
			if (process.argv[2] === 'cancel') { waitingPermission = message.id; }
			else {
				send({ jsonrpc: '2.0', method: 'session/request_permission', id: 'permission-1', params: { sessionId, toolCall: { toolCallId: 'tool-1' }, options: [{ optionId: 'allow', name: 'Allow', kind: 'allow_once' }, { optionId: 'deny', name: 'Reject', kind: 'reject_once' }] } });
				waitingPermission = { promptId: message.id, prompt: message.params.prompt[0].text };
			}
		} else if (message.method === 'session/cancel') {
			if (waitingPermission) { result(waitingPermission, { stopReason: 'cancelled' }); waitingPermission = undefined; }
		} else if (message.id === 'permission-1') {
			const summary = `cwd=${sessionParams.cwd};caps=${JSON.stringify(initializeParams.clientCapabilities)};mcp=${JSON.stringify(sessionParams.mcpServers)};argv=${JSON.stringify(process.argv.slice(3))};auth=${authMethod || ''};prompt=${waitingPermission.prompt};permission=${JSON.stringify(message.result.outcome)}`;
			send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId, update: { sessionUpdate: 'agent_message_chunk', messageId: 'message-1', content: { type: 'text', text: summary } } } });
			result(waitingPermission.promptId, { stopReason: 'end_turn' });
			waitingPermission = undefined;
		} else if (message.method === 'terminal/create') {
			result(message.id, { rejected: true });
		}
	}
});
