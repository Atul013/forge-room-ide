export type GitCommitSummary = { hash: string; message: string; parents?: string[] };

export function mergeCommitActivity(commit: GitCommitSummary) {
	if (!Array.isArray(commit.parents) || commit.parents.length < 2) { return undefined; }
	const hash = commit.hash.slice(0, 8);
	const subject = commit.message.split(/\r?\n/, 1)[0];
	return `${hash} ${subject}`;
}
