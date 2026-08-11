import { strictEqual } from "node:assert/strict";
import { describe, test } from "node:test";
import {
	isOptionalWholeNumber,
	parseOptionalWholeNumber,
} from "./library-form-validation";

describe("library form whole-number fields", () => {
	test("accepts blank, trimmed, and bounded unsigned integers", () => {
		strictEqual(isOptionalWholeNumber("", 65535), true);
		strictEqual(isOptionalWholeNumber(" 2024 ", 65535), true);
		strictEqual(isOptionalWholeNumber("0003", 255), true);
		strictEqual(parseOptionalWholeNumber(" 0003 ", 255), 3);
	});

	test("rejects partial, decimal, signed, and out-of-range values", () => {
		for (const value of ["2024abc", "3.5", "+3", "-1"]) {
			strictEqual(isOptionalWholeNumber(value, 65535), false, value);
			strictEqual(parseOptionalWholeNumber(value, 65535), undefined, value);
		}
		strictEqual(isOptionalWholeNumber("65536", 65535), false);
		strictEqual(isOptionalWholeNumber("256", 255), false);
	});
});
