const WHOLE_NUMBER = /^\d+$/;

export function isOptionalWholeNumber(value: string, max: number): boolean {
	const trimmed = value.trim();
	if (trimmed === "") return true;
	if (!WHOLE_NUMBER.test(trimmed)) return false;
	const parsed = Number(trimmed);
	return Number.isSafeInteger(parsed) && parsed <= max;
}

export function parseOptionalWholeNumber(
	value: string,
	max: number,
): number | undefined {
	const trimmed = value.trim();
	if (!trimmed || !isOptionalWholeNumber(trimmed, max)) return undefined;
	return Number(trimmed);
}
