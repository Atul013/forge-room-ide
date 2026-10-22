let input = '';

function send(message) {
	process.stdout.write(JSON.stringify(message) + '\n');
}

process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => {
	input += chunk;
	for (;;) {
		const newline = input.indexOf('\n');
		if (newline < 0) { break; }
		const line = input.slice(0, newline).trim();
		input = input.slice(newline + 1);
		if (!line) { continue; }
		let message;
		try { message = JSON.parse(line); } catch { continue; }
		if (message.method === 'initialize') {
			if (process.argv.includes('--stall-initialize')) { continue; }
			send({ jsonrpc: '2.0', id: message.id, result: { protocolVersion: message.params.protocolVersion, capabilities: { tools: {} }, serverInfo: { name: 'forge-room-test', version: '1.0.0' } } });
		} else if (message.method === 'tools/list') {
			if (process.argv.includes('--stall-list')) { continue; }
			send({ jsonrpc: '2.0', id: message.id, result: { tools: [{ name: 'echo', description: 'Return a test value', inputSchema: { type: 'object', properties: { value: { type: 'string' } }, required: ['value'], additionalProperties: false } }] } });
		} else if (message.method === 'tools/call') {
			if (process.argv.includes('--stall-call')) { continue; }
			send({ jsonrpc: '2.0', id: message.id, result: { content: [{ type: 'text', text: `echo:${message.params.arguments.value}` }] } });
		}
	}
});
