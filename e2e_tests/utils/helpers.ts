/**
 * Concats an arbitrary number of Uint8Array's
 * @param {Uint8Array[]} ...arrays - The arrays to be concatenated.
 * @returns {Uint8Array} -The concatenation of the arrays.
 */
export function concatUint8Arrays(...arrays: Uint8Array[]): Uint8Array {
	let totalLength = arrays.reduce((acc, curr) => acc + curr.length, 0);
	let result = new Uint8Array(totalLength);
	let offset = 0;
	for (let arr of arrays) {
		result.set(arr, offset);
		offset += arr.length;
	}
	return result;
}
