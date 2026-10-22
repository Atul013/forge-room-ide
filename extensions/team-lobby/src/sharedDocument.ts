import * as Y from 'yjs';

export const maxSharedDocumentBytes = 512 * 1024;
export const maxSharedDocumentUpdateBytes = 512 * 1024;
export const maxSharedDocumentStateBytes = 900 * 1024;
const maxSharedPathLength = 500;

export type SharedDocumentUpdate = { file: string; update: string };
export type SharedDocumentRelay = { update: string; recipients: string[] };

/** Validate subscription membership before applying an incoming peer update. */
export function routeSharedDocumentUpdate(document: SharedTextDocument, subscribers: Iterable<string>, senderId: string, update: unknown): SharedDocumentRelay | undefined {
	const recipients = [...new Set(subscribers)];
	if (!senderId || !recipients.includes(senderId) || typeof update !== 'string' || !document.apply(update)) { return undefined; }
	return { update, recipients: recipients.filter(id => id !== senderId) };
}

export function validSharedDocumentPath(value: unknown): value is string {
	return typeof value === 'string' && value.length > 0 && value.length <= maxSharedPathLength && !/[\u0000-\u001f\u007f]/.test(value) && !value.startsWith('/') && !/^[a-zA-Z]:/.test(value) && !value.replace(/\\/g, '/').split('/').some(part => !part || part === '.' || part === '..');
}

export function decodeSharedDocumentUpdate(value: unknown, maximumBytes = maxSharedDocumentUpdateBytes): Uint8Array | undefined {
	if (typeof value !== 'string' || value.length === 0 || value.length > Math.ceil(maximumBytes * 4 / 3) || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) { return undefined; }
	const bytes = Buffer.from(value, 'base64');
	return bytes.length > 0 && bytes.length <= maximumBytes ? new Uint8Array(bytes) : undefined;
}

/** Holds one room-scoped collaborative text document. State is intentionally in-memory only. */
export class SharedTextDocument {
	private readonly doc = new Y.Doc();
	private readonly text = this.doc.getText('source');
	private applyingRemote = false;
	private readonly updateListeners = new Set<(update: string, remote: boolean) => void>();
	private readonly handleUpdate = (update: Uint8Array, origin: unknown) => {
		const encoded = Buffer.from(update).toString('base64');
		for (const listener of this.updateListeners) { listener(encoded, origin === 'remote'); }
	};

	constructor(readonly file: string, initialText = '') {
		if (!validSharedDocumentPath(file)) { throw new Error('Shared document path must be workspace-relative.'); }
		if (Buffer.byteLength(initialText, 'utf8') > maxSharedDocumentBytes) { throw new Error('Shared document exceeds the 512 KB limit.'); }
		if (initialText) { this.text.insert(0, initialText); }
		this.doc.on('update', this.handleUpdate);
	}

	get value(): string { return this.text.toString(); }
	get isApplyingRemoteUpdate(): boolean { return this.applyingRemote; }
	encode(): string { return Buffer.from(Y.encodeStateAsUpdate(this.doc)).toString('base64'); }
	onUpdate(listener: (update: string, remote: boolean) => void): { dispose(): void } {
		this.updateListeners.add(listener);
		return { dispose: () => this.updateListeners.delete(listener) };
	}

	/** Apply a validated peer update; malformed updates leave the document unchanged. */
	apply(encoded: unknown, maximumBytes = maxSharedDocumentUpdateBytes): boolean {
		const update = decodeSharedDocumentUpdate(encoded, maximumBytes);
		if (!update) { return false; }
		try {
			const candidate = new Y.Doc();
			let withinDocumentLimit = false;
			try {
				Y.applyUpdate(candidate, Y.encodeStateAsUpdate(this.doc));
				Y.applyUpdate(candidate, update);
				withinDocumentLimit = Buffer.byteLength(candidate.getText('source').toString(), 'utf8') <= maxSharedDocumentBytes;
			} finally { candidate.destroy(); }
			if (!withinDocumentLimit) { return false; }
			this.applyingRemote = true;
			Y.applyUpdate(this.doc, update, 'remote');
			return true;
		} catch {
			return false;
		} finally { this.applyingRemote = false; }
	}

	applyState(encoded: unknown): boolean { return this.apply(encoded, maxSharedDocumentStateBytes); }

	/** Capture an editor change as a Yjs operation using pre-change document offsets. */
	edit(offset: number, deleteLength: number, insertText: string): boolean {
		return this.editMany([{ offset, deleteLength, insertText }]);
	}

	/** Apply one editor event atomically; offsets refer to the same pre-change version. */
	editMany(changes: Array<{ offset: number; deleteLength: number; insertText: string }>): boolean {
		if (!changes.length) { return true; }
		const current = this.value;
		const ordered = [...changes].sort((a, b) => b.offset - a.offset);
		for (let index = 0; index < ordered.length; index++) {
			const change = ordered[index];
			if (typeof change.insertText !== 'string' || !Number.isSafeInteger(change.offset) || !Number.isSafeInteger(change.deleteLength) || change.offset < 0 || change.deleteLength < 0 || change.offset + change.deleteLength > current.length || index > 0 && change.offset + change.deleteLength > ordered[index - 1].offset) { return false; }
		}
		let next = current;
		for (const change of ordered) { next = next.slice(0, change.offset) + change.insertText + next.slice(change.offset + change.deleteLength); }
		if (Buffer.byteLength(next, 'utf8') > maxSharedDocumentBytes) { return false; }
		this.doc.transact(() => {
			for (const change of ordered) {
				if (change.deleteLength) { this.text.delete(change.offset, change.deleteLength); }
				if (change.insertText) { this.text.insert(change.offset, change.insertText); }
			}
		}, 'local');
		return true;
	}

	destroy(): void { this.doc.off('update', this.handleUpdate); this.updateListeners.clear(); this.doc.destroy(); }
}
