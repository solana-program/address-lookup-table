/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/metaplex-foundation/kinobi
 */

export const enum AcmeCounterProgramErrorCode {
  /** DeserializationError: Error deserializing an account */
  DESERIALIZATION_ERROR = 0x0, // 0
  /** SerializationError: Error serializing an account */
  SERIALIZATION_ERROR = 0x1, // 1
  /** InvalidProgramOwner: Invalid program owner. This likely mean the provided account does not exist */
  INVALID_PROGRAM_OWNER = 0x2, // 2
  /** InvalidPda: Invalid PDA derivation */
  INVALID_PDA = 0x3, // 3
  /** ExpectedEmptyAccount: Expected empty account */
  EXPECTED_EMPTY_ACCOUNT = 0x4, // 4
  /** ExpectedNonEmptyAccount: Expected non empty account */
  EXPECTED_NON_EMPTY_ACCOUNT = 0x5, // 5
  /** ExpectedSignerAccount: Expected signer account */
  EXPECTED_SIGNER_ACCOUNT = 0x6, // 6
  /** ExpectedWritableAccount: Expected writable account */
  EXPECTED_WRITABLE_ACCOUNT = 0x7, // 7
  /** AccountMismatch: Account mismatch */
  ACCOUNT_MISMATCH = 0x8, // 8
  /** InvalidAccountKey: Invalid account key */
  INVALID_ACCOUNT_KEY = 0x9, // 9
  /** NumericalOverflow: Numerical overflow */
  NUMERICAL_OVERFLOW = 0xa, // 10
}

export class AcmeCounterProgramError extends Error {
  override readonly name = 'AcmeCounterProgramError';

  readonly code: AcmeCounterProgramErrorCode;

  readonly cause: Error | undefined;

  constructor(
    code: AcmeCounterProgramErrorCode,
    name: string,
    message: string,
    cause?: Error
  ) {
    super(`${name} (${code}): ${message}`);
    this.code = code;
    this.cause = cause;
  }
}

let acmeCounterProgramErrorCodeMap:
  | Record<AcmeCounterProgramErrorCode, [string, string]>
  | undefined;
if (__DEV__) {
  acmeCounterProgramErrorCodeMap = {
    [AcmeCounterProgramErrorCode.DESERIALIZATION_ERROR]: [
      'DeserializationError',
      `Error deserializing an account`,
    ],
    [AcmeCounterProgramErrorCode.SERIALIZATION_ERROR]: [
      'SerializationError',
      `Error serializing an account`,
    ],
    [AcmeCounterProgramErrorCode.INVALID_PROGRAM_OWNER]: [
      'InvalidProgramOwner',
      `Invalid program owner. This likely mean the provided account does not exist`,
    ],
    [AcmeCounterProgramErrorCode.INVALID_PDA]: [
      'InvalidPda',
      `Invalid PDA derivation`,
    ],
    [AcmeCounterProgramErrorCode.EXPECTED_EMPTY_ACCOUNT]: [
      'ExpectedEmptyAccount',
      `Expected empty account`,
    ],
    [AcmeCounterProgramErrorCode.EXPECTED_NON_EMPTY_ACCOUNT]: [
      'ExpectedNonEmptyAccount',
      `Expected non empty account`,
    ],
    [AcmeCounterProgramErrorCode.EXPECTED_SIGNER_ACCOUNT]: [
      'ExpectedSignerAccount',
      `Expected signer account`,
    ],
    [AcmeCounterProgramErrorCode.EXPECTED_WRITABLE_ACCOUNT]: [
      'ExpectedWritableAccount',
      `Expected writable account`,
    ],
    [AcmeCounterProgramErrorCode.ACCOUNT_MISMATCH]: [
      'AccountMismatch',
      `Account mismatch`,
    ],
    [AcmeCounterProgramErrorCode.INVALID_ACCOUNT_KEY]: [
      'InvalidAccountKey',
      `Invalid account key`,
    ],
    [AcmeCounterProgramErrorCode.NUMERICAL_OVERFLOW]: [
      'NumericalOverflow',
      `Numerical overflow`,
    ],
  };
}

export function getAcmeCounterProgramErrorFromCode(
  code: AcmeCounterProgramErrorCode,
  cause?: Error
): AcmeCounterProgramError {
  if (__DEV__) {
    return new AcmeCounterProgramError(
      code,
      ...(
        acmeCounterProgramErrorCodeMap as Record<
          AcmeCounterProgramErrorCode,
          [string, string]
        >
      )[code],
      cause
    );
  }

  return new AcmeCounterProgramError(
    code,
    'Unknown',
    'Error message not available in production bundles. Compile with __DEV__ set to true to see more information.',
    cause
  );
}
