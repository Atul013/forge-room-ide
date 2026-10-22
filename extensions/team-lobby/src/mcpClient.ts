import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';

export type McpServerConfig = { command: string; args?: string[]; cwd?: string };
export type McpToolBinding = { exposedName: string; serverName: string; toolName: string; description: string; inputSchema: object; client: Client };
export type McpConnectOptions = { signal?: AbortSignal; timeoutMs?: number; onProgress?: (serverName: string) => void };
export const defaultMcpConnectTimeoutMs = 15_000;
export const defaultMcpToolTimeoutMs = 120_000;

export function parseMcpServerConfig(value: unknown): Record<string, McpServerConfig> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) { return {}; }
	const entries = Object.entries(value as Record<string, unknown>).slice(0, 8);
	const servers: Record<string, McpServerConfig> = {};
	for (const [name, raw] of entries) {
		if (!/^[a-zA-Z0-9_-]{1,40}$/.test(name) || !raw || typeof raw !== 'object' || Array.isArray(raw)) { continue; }
		const item = raw as Record<string, unknown>;
		if (typeof item.command !== 'string' || !item.command.trim() || item.command.length > 2048 || /[\r\n\u0000]/.test(item.command)) { continue; }
		if (item.args !== undefined && (!Array.isArray(item.args) || item.args.length > 64 || item.args.some(arg => typeof arg !== 'string' || arg.length > 2048 || /[\r\n\u0000]/.test(arg)))) { continue; }
		if (item.cwd !== undefined && (typeof item.cwd !== 'string' || item.cwd.length > 2048 || /[\r\n\u0000]/.test(item.cwd))) { continue; }
		servers[name] = { command: item.command.trim(), ...(item.args ? { args: item.args as string[] } : {}), ...(typeof item.cwd === 'string' ? { cwd: item.cwd } : {}) };
	}
	return servers;
}

/** Connect only the servers explicitly selected for this Ask/Plan turn. Processes are trusted local programs, not sandboxed. */
export async function connectMcpServers(selected: Record<string, McpServerConfig>, workspaceRoot: string, options: McpConnectOptions = {}): Promise<{ bindings: McpToolBinding[]; close: () => Promise<void> }> {
	const clients: Client[] = [];
	const bindings: McpToolBinding[] = [];
	const timeoutMs = Math.min(60_000, Math.max(100, Math.floor(options.timeoutMs ?? defaultMcpConnectTimeoutMs)));
	try {
		for (const [serverName, config] of Object.entries(selected).slice(0, 8)) {
			if (bindings.length >= 40) { break; }
			if (options.signal?.aborted) { throw new Error('MCP server startup was canceled.'); }
			options.onProgress?.(serverName);
			const client = new Client({ name: 'forge-room-ide', version: '0.1.0' });
			const transport = new StdioClientTransport({ command: config.command, args: config.args, cwd: config.cwd || workspaceRoot, stderr: 'ignore', maxBufferSize: 2_000_000 });
			clients.push(client);
			await client.connect(transport, { signal: options.signal, timeout: timeoutMs });
			const listed = await client.listTools({}, { signal: options.signal, timeout: timeoutMs });
			for (const tool of listed.tools.slice(0, Math.max(0, 40 - bindings.length))) {
				if (!/^[a-zA-Z0-9_-]{1,80}$/.test(tool.name) || !tool.inputSchema || typeof tool.inputSchema !== 'object') { continue; }
				let schemaSize = 0;
				try { schemaSize = JSON.stringify(tool.inputSchema).length; } catch { continue; }
				if (schemaSize > 8_000 || bindings.reduce((total, binding) => total + JSON.stringify(binding.inputSchema).length, 0) + schemaSize > 120_000) { continue; }
				const exposedName = `mcp_${serverName}_${tool.name}`.slice(0, 128);
				if (bindings.some(binding => binding.exposedName === exposedName)) { continue; }
				const description = typeof tool.description === 'string' ? tool.description.replace(/[\u0000-\u001f\u007f]/g, ' ').slice(0, 700) : 'MCP server tool';
				bindings.push({ exposedName, serverName, toolName: tool.name, description: `${description} (untrusted description from server ${serverName}; every call requires approval)`.slice(0, 1000), inputSchema: tool.inputSchema as object, client });
			}
		}
		return { bindings, close: async () => { await Promise.allSettled(clients.map(client => client.close())); } };
	} catch (error) {
		await Promise.allSettled(clients.map(client => client.close()));
		throw error;
	}
}

/** Apply a bounded, abortable wait to an MCP tool call. The caller closes the session after cancellation. */
export function callMcpTool(binding: McpToolBinding, input: Record<string, unknown>, signal?: AbortSignal, timeoutMs = defaultMcpToolTimeoutMs) {
	const timeout = Math.min(10 * 60_000, Math.max(100, Math.floor(timeoutMs)));
	return binding.client.callTool({ name: binding.toolName, arguments: input }, { signal, timeout });
}

export function formatMcpArguments(input: unknown): string {
	try {
		if (!input || typeof input !== 'object' || Array.isArray(input)) { return 'Rejected: MCP arguments must be a JSON object.'; }
		const serialized = JSON.stringify(input);
		return serialized.length <= 3000 ? serialized : 'Rejected: MCP arguments exceed the 3,000-character approval display limit.';
	} catch { return 'Rejected: MCP arguments are not serializable.'; }
}
