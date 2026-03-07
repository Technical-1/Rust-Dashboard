export function logError(context: string, e: unknown) {
	if (import.meta.env.DEV) {
		console.error(`${context}:`, e);
	}
}
