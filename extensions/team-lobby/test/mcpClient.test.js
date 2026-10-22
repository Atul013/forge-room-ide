const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { parseMcpServerConfig, formatMcpArguments, connectMcpServers, callMcpTool } = require('../dist/mcpClient');

test('MCP settings accept bounded stdio command entries and discard malformed entries', () => {
	const parsed = parseMcpServerConfig({
		filesystem: { command: 'node', args: ['server.js'], cwd: 'C:/project' },
		badName: { command: ' node\n--danger' },
		badArgs: { command: 'node', args: ['ok', 4] },
		badCwd: { command: 'node', cwd: 'bad\0path' },
		badShape: 'node'
	});
	assert.deepEqual(parsed, { filesystem: { command: 'node', args: ['server.js'], cwd: 'C:/project' } });
});

test('MCP settings cap configured server count and reject unsafe setting names', () => {
	const input = Object.fromEntries(Array.from({ length: 10 }, (_, index) => [`server_${index}`, { command: 'node' }]));
	input['space name'] = { command: 'node' };
	assert.equal(Object.keys(parseMcpServerConfig(input)).length, 8);
	assert.deepEqual(parseMcpServerConfig(null), {});
});

test('MCP argument previews stay bounded and serialize safely', () => {
	assert.equal(formatMcpArguments({ x: 1 }), '{"x":1}');
	assert.match(formatMcpArguments({ x: 'x'.repeat(3000) }), /^Rejected:.*3,000-character/);
	assert.match(formatMcpArguments(undefined), /^Rejected:.*JSON object/);
	assert.match(formatMcpArguments([]), /^Rejected:.*JSON object/);
});

test('MCP SDK connects to a stdio server, lists a tool, calls it, and closes the process', async () => {
	const fixture = path.join(__dirname, 'fixtures', 'mockMcpServer.js');
	const session = await connectMcpServers({ test: { command: process.execPath, args: [fixture] } }, process.cwd());
	try {
		assert.equal(session.bindings.length, 1);
		assert.equal(session.bindings[0].exposedName, 'mcp_test_echo');
		const response = await session.bindings[0].client.callTool({ name: 'echo', arguments: { value: 'works' } });
		assert.equal(response.isError, undefined);
		assert.equal(response.content[0].text, 'echo:works');
	} finally {
		await session.close();
	}
});

test('MCP server startup and discovery stop waiting when a server does not answer', async () => {
	const fixture = path.join(__dirname, 'fixtures', 'mockMcpServer.js');
	for (const mode of ['--stall-initialize', '--stall-list']) {
		await assert.rejects(
			connectMcpServers({ stalled: { command: process.execPath, args: [fixture, mode] } }, process.cwd(), { timeoutMs: 100 }),
			/timeout|timed out/i
		);
	}
});

test('MCP tool calls use an abortable timeout bound', async () => {
	const fixture = path.join(__dirname, 'fixtures', 'mockMcpServer.js');
	const session = await connectMcpServers({ test: { command: process.execPath, args: [fixture, '--stall-call'] } }, process.cwd());
	try {
		await assert.rejects(callMcpTool(session.bindings[0], { value: 'wait' }, undefined, 100), /timeout|timed out/i);
	} finally { await session.close(); }
});
