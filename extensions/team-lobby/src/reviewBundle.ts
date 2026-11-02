/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

const maxReviewRiskCharacters = 2_000;

export function formatReviewRisks(value: string): string {
	const normalized = value
		.replace(/\r\n?/g, '\n')
		.replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/g, '')
		.slice(0, maxReviewRiskCharacters)
		.trim();
	if (!normalized) { return '> No additional risks recorded.'; }
	return [ '> User-entered reviewer notes; not independently verified.', ...normalized.split('\n').map(line => `> ${line}`) ].join('\n');
}
