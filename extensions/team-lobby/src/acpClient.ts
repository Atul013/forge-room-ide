import { spawn } from 'node:child_process';

export type AcpAgentResult = { text: string; agentName?: string; agentVersion?: string; sessionId?: string; sessionResumable?: boolean; resumedSession?: boolean; authMethodId?: string; stopReason?: string };
export type AcpAuthMethod = { id: string; name: string; description?: string; type?: string };
type RpcMessage = { jsonrpc?: string; id?: string | number | null; method?: string; params?: any; result?: any; error?: { code?: number; message?: string } };
type PendingRequest = { resolve: (value: any) => void; reject: (error: Error) => void; timer: NodeJS.Timeout };

const maxLineLength = 1_000_000;
const maxOutputLength = 200_000;

/** Run a trusted, user-configured ACP v1 stdio agent for one Ask/Plan turn. */
export async function runAcpAgent(command: string, args: string[], cwd: string, prompt: string, options: {
	cancelled: () => boolean;
	onProgress?: (message: string) => void;
	onText?: (chunk: string) => void;
	selectAuthenticationMethod?: (methods: AcpAuthMethod[]) => Promise<string | undefined>;
	previousSessionId?: string;
	previousAuthMethodId?: string;
	selectPreviousSession?: (availability: { canLoad: boolean; authMatches: boolean }) => Promise<'resume' | 'new' | undefined>;
}): Promise<AcpAgentResult> {
	if (!command.trim() || command.length > 2048 || /[\r\n\u0000]/.test(command) || !Array.isArray(args) || args.length > 64 || args.some(arg => typeof arg !== 'string' || arg.length > 2048 || /[\r\n\u0000]/.test(arg))) { throw new Error('The configured ACP command or arguments are invalid.'); }
	if (!cwd || !prompt.trim() || prompt.length > 1_000_000) { throw new Error('The selected workspace or prompt is invalid or too large for ACP.'); }
	if (options.cancelled()) { throw new Error('The ACP request was canceled before the agent started.'); }
	const child = spawn(command, args, { cwd, windowsHide: true, shell: false, stdio: ['pipe', 'pipe', 'ignore'] });
	const pending = new Map<string, PendingRequest>();
	let nextId = 1;
	let buffer = '';
	let responseText = '';
	let sessionId: string | undefined;
	let replayingSession = false;
	let agentName: string | undefined;
	let agentVersion: string | undefined;
	let authMethodId: string | undefined;
	let canceled = false;
	let promptStarted = false;
	let settled = false;
	let rejectProcess!: (error: Error) => void;
	const processFailure = new Promise<never>((_resolve, reject) => { rejectProcess = reject; });
	const disposePending = (error: Error) => {
		for (const request of pending.values()) { clearTimeout(request.timer); request.reject(error); }
		pending.clear();
	};
	const fail = (error: Error) => {
		if (settled) { return; }
		settled = true;
		disposePending(error);
		rejectProcess(error);
	};
	const send = (message: Record<string, unknown>) => {
		if (child.stdin.destroyed || child.stdin.writableEnded) { throw new Error('The ACP agent closed its input stream.'); }
		child.stdin.write(`${JSON.stringify(message)}\n`);
	};
	const sendResponse = (id: string | number | null, result: unknown) => send({ jsonrpc: '2.0', id, result });
	const sendError = (id: string | number | null, code: number, message: string) => send({ jsonrpc: '2.0', id, error: { code, message } });
	const request = (method: string, params: unknown, timeoutMs = 30_000): Promise<any> => {
		const id = nextId++;
		const key = String(id);
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => { pending.delete(key); reject(new Error(`The ACP agent did not answer ${method} before its timeout.`)); }, timeoutMs);
			pending.set(key, { resolve, reject, timer });
			try { send({ jsonrpc: '2.0', id, method, params }); }
			catch (error) { clearTimeout(timer); pending.delete(key); reject(error instanceof Error ? error : new Error(String(error))); }
		});
	};
	const handleMessage = (message: RpcMessage) => {
		if (!message || message.jsonrpc !== '2.0') { fail(new Error('The ACP agent sent an invalid JSON-RPC message.')); return; }
		if (message.method) {
			if (message.method === 'session/update') {
				if (!sessionId || message.params?.sessionId !== sessionId || replayingSession) { return; }
				const update = message.params?.update;
				if (update?.sessionUpdate === 'agent_message_chunk' && update.content?.type === 'text' && typeof update.content.text === 'string') {
					const remaining = maxOutputLength - responseText.length;
					if (remaining <= 0) { return; }
					const chunk = update.content.text.slice(0, remaining);
					responseText += chunk;
					options.onText?.(chunk);
				} else if (update?.sessionUpdate === 'plan' && Array.isArray(update.entries)) {
					const summary = update.entries.slice(0, 12).map((entry: any) => typeof entry?.content === 'string' ? entry.content.slice(0, 160) : '').filter(Boolean).join('; ');
					if (summary) { options.onProgress?.(`ACP plan: ${summary.slice(0, 800)}`); }
				} else if (update?.sessionUpdate === 'tool_call' || update?.sessionUpdate === 'tool_call_update') {
					options.onProgress?.('External ACP agent is using its own tool. Forge Room does not control that tool through the client API.');
				}
				return;
			}
			if (message.method === 'session/request_permission' && message.id !== undefined && message.id !== null) {
				const optionsList = Array.isArray(message.params?.options) ? message.params.options : [];
				const rejectOption = optionsList.find((option: any) => option?.kind === 'reject_once' && typeof option.optionId === 'string');
				if (options.cancelled()) { sendResponse(message.id, { outcome: { outcome: 'cancelled' } }); }
				else if (rejectOption) { sendResponse(message.id, { outcome: { outcome: 'selected', optionId: rejectOption.optionId } }); }
				else { sendResponse(message.id, { outcome: { outcome: 'cancelled' } }); }
				options.onProgress?.('An external ACP permission request was denied.');
				return;
			}
			if (message.id !== undefined && message.id !== null) { sendError(message.id, -32601, 'Forge Room has not advertised or implemented this ACP client method.'); }
			return;
		}
		if (message.id === undefined || message.id === null) { return; }
		const key = String(message.id);
		const waiting = pending.get(key);
		if (!waiting) { return; }
		pending.delete(key);
		clearTimeout(waiting.timer);
		if (message.error) { waiting.reject(new Error(`ACP ${message.error.code ?? 'error'}: ${(message.error.message || 'request failed').slice(0, 500)}`)); }
		else { waiting.resolve(message.result); }
	};
	child.stdout.setEncoding('utf8');
	child.stdout.on('data', (chunk: string) => {
		buffer += chunk;
		if (buffer.length > maxLineLength && !buffer.includes('\n')) { fail(new Error('The ACP agent sent an oversized protocol message.')); child.kill(); return; }
		let newline: number;
		while ((newline = buffer.indexOf('\n')) >= 0) {
			const line = buffer.slice(0, newline).trim();
			buffer = buffer.slice(newline + 1);
			if (!line) { continue; }
			if (line.length > maxLineLength) { fail(new Error('The ACP agent sent an oversized protocol message.')); child.kill(); return; }
			try { handleMessage(JSON.parse(line) as RpcMessage); }
			catch { fail(new Error('The ACP agent sent malformed JSON-RPC.')); child.kill(); return; }
		}
	});
	child.on('error', error => fail(new Error(`Could not start the configured ACP agent: ${error.message}`)));
	child.on('exit', (code, signal) => {
		if (!settled) { fail(new Error(`The ACP agent exited before completing the request (${code === null ? signal || 'unknown' : `exit ${code}`}).`)); }
	});
	const cancel = () => {
		if (canceled || settled) { return; }
		canceled = true;
		if (sessionId && promptStarted) { try { send({ jsonrpc: '2.0', method: 'session/cancel', params: { sessionId } }); } catch { /* The process may already be exiting. */ } }
		setTimeout(() => { if (!settled) { child.kill(); } }, 2000).unref();
	};
	const cancellationTimer = setInterval(() => { if (options.cancelled()) { cancel(); } }, 100);
	try {
		const initialized = await Promise.race([
			request('initialize', { protocolVersion: 1, clientCapabilities: {}, clientInfo: { name: 'forge-room', title: 'Forge Room IDE', version: '0.76.0' } }),
			processFailure
		]);
		if (initialized?.protocolVersion !== 1) { throw new Error(`The ACP agent negotiated unsupported protocol version ${String(initialized?.protocolVersion)}.`); }
		if (Array.isArray(initialized.authMethods) && initialized.authMethods.length > 0) {
			const methods = initialized.authMethods.filter((method: any): method is AcpAuthMethod => method && typeof method.id === 'string' && method.id.length > 0 && method.id.length <= 200 && typeof method.name === 'string' && method.name.length > 0 && method.name.length <= 200 && (method.type === undefined || method.type === 'agent')).map((method: AcpAuthMethod) => ({ id: method.id, name: method.name, description: typeof method.description === 'string' ? method.description.slice(0, 500) : undefined, type: method.type }));
			if (!methods.length) { throw new Error('The ACP agent requires terminal authentication, which Forge Room does not support. Authenticate it using its own trusted setup before connecting it to Forge Room.'); }
			const selectedId = await options.selectAuthenticationMethod?.(methods);
			if (!selectedId || !methods.some((method: AcpAuthMethod) => method.id === selectedId)) { throw new Error('ACP authentication was canceled.'); }
			authMethodId = selectedId;
			options.onProgress?.(`Authenticating with external ACP agent using ${methods.find((method: AcpAuthMethod) => method.id === selectedId)!.name}.`);
			await Promise.race([request('authenticate', { methodId: selectedId }), processFailure]);
		}
		if (options.cancelled()) { throw new Error('The ACP request was canceled.'); }
		agentName = typeof initialized.agentInfo?.title === 'string' ? initialized.agentInfo.title.slice(0, 100) : typeof initialized.agentInfo?.name === 'string' ? initialized.agentInfo.name.slice(0, 100) : undefined;
		agentVersion = typeof initialized.agentInfo?.version === 'string' ? initialized.agentInfo.version.slice(0, 60) : undefined;
		const sessionResumable = initialized.agentCapabilities?.loadSession === true;
		let resumedSession = false;
		if (options.previousSessionId && typeof options.previousSessionId === 'string' && options.previousSessionId.length <= 500) {
			const authMatches = options.previousAuthMethodId === authMethodId;
			const canLoad = sessionResumable && authMatches;
			const choice = options.selectPreviousSession ? await options.selectPreviousSession({ canLoad, authMatches }) : 'new';
			if (!choice) { throw new Error('ACP session selection was canceled.'); }
			if (choice === 'resume' && canLoad) {
				sessionId = options.previousSessionId;
				replayingSession = true;
				options.onProgress?.('Restoring the external agent conversation history.');
				await Promise.race([request('session/load', { sessionId, cwd, mcpServers: [] }), processFailure]);
				replayingSession = false;
				resumedSession = true;
			} else if (choice === 'resume') {
				throw new Error(authMatches ? 'The configured ACP agent does not support loading previous sessions.' : 'The saved ACP session used a different authentication method.');
			}
		}
		if (!resumedSession) {
			const created = await Promise.race([request('session/new', { cwd, mcpServers: [] }), processFailure]);
			if (typeof created?.sessionId !== 'string' || !created.sessionId || created.sessionId.length > 500) { throw new Error('The ACP agent returned an invalid session ID.'); }
			sessionId = created.sessionId;
		}
		if (options.cancelled()) { throw new Error('The ACP request was canceled.'); }
		promptStarted = true;
		const promptRequest = request('session/prompt', { sessionId, prompt: [{ type: 'text', text: prompt }] }, 10 * 60_000);
		const promptResult = await Promise.race([promptRequest, processFailure]);
		if (canceled || options.cancelled()) { throw new Error('The ACP request was canceled.'); }
		if (!responseText.trim()) { throw new Error('The ACP agent completed without returning a text response.'); }
		return { text: responseText, agentName, agentVersion, sessionId, sessionResumable, resumedSession, authMethodId, stopReason: typeof promptResult?.stopReason === 'string' ? promptResult.stopReason : undefined };
	} catch (error) {
		if (sessionId && promptStarted && !canceled) { try { send({ jsonrpc: '2.0', method: 'session/cancel', params: { sessionId } }); } catch { /* The process may already be exiting. */ } }
		child.kill();
		throw error instanceof Error ? error : new Error(String(error));
	} finally {
		clearInterval(cancellationTimer);
		settled = true;
		disposePending(new Error('The ACP session ended.'));
		if (!child.killed) { child.kill(); }
	}
}
