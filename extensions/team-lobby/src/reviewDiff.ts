export type ReviewDiffFile = { path: string; before?: string; after: string; created?: boolean };

export function unifiedReviewDiff(files: ReviewDiffFile[]) {
	return files.map(file => {
		const relative = file.path.replace(/\\/g, '/').replace(/\r/g, '%0D').replace(/\n/g, '%0A').replace(/\t/g, '%09');
		const oldText = file.before || '';
		const newText = file.after;
		const lines = (text: string) => text === '' ? [] : text.replace(/\r\n/g, '\n').split('\n').filter((_line, index, all) => !(index === all.length - 1 && all[index] === ''));
		const before = lines(oldText);
		const after = lines(newText);
		const oldRange = before.length ? `1,${before.length}` : '0,0';
		const newRange = after.length ? `1,${after.length}` : '0,0';
		const removed = before.map(line => `-${line}`);
		const added = after.map(line => `+${line}`);
		const noNewline = String.fromCharCode(92) + ' No newline at end of file';
		if (oldText && !oldText.endsWith('\n')) { removed.push(noNewline); }
		if (newText && !newText.endsWith('\n')) { added.push(noNewline); }
		return [`diff --git a/${relative} b/${relative}`, ...(file.created ? ['new file mode 100644'] : []), `--- ${file.created ? '/dev/null' : `a/${relative}`}`, `+++ b/${relative}`, `@@ -${oldRange} +${newRange} @@`, ...removed, ...added].join('\n');
	}).join('\n\n');
}
