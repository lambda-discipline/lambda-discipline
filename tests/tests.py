from os.path import dirname, join
import subprocess
import json
import logger

TESTS = dirname(__file__)
ROOT = dirname(TESTS)
BUILD = join(ROOT, "cmake-build-debug")
EXE = join(BUILD, "lbd")
TEST_CASES = join(TESTS, "test_cases.json")

TEST_CASE_NAME_KEY = "name"
TEST_CASE_SOURCE_KEY = "source"
TEST_CASE_OUTPUT_KEY = "output"


# TODO: Also log the time taken.
# TODO: Add tests for REPL.

def execCommand(command):
    return subprocess.run(command, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)


if __name__ == "__main__":
    with open(TEST_CASES, mode="r", encoding="utf-8") as file:
        testCases = json.load(file)
    passCount, failCount, totalCount = 0, 0, 0
    for testCase in testCases:
        if TEST_CASE_SOURCE_KEY not in testCase or TEST_CASE_OUTPUT_KEY not in testCase:
            logger.error("Invalid test-case.")
            continue
        testName = testCase[TEST_CASE_NAME_KEY] if TEST_CASE_NAME_KEY in testCase else testCase[TEST_CASE_SOURCE_KEY]
        result = execCommand([EXE, "--file", testCase[TEST_CASE_SOURCE_KEY]])
        if (exitCode := result.returncode) != 0:
            logger.error(f"Test failed '{testName}' due to non-zero exit-code '{exitCode}'.")
        with open(join(ROOT, testCase[TEST_CASE_OUTPUT_KEY]), mode="r") as file:
            expectedStdout = file.read()
        totalCount += 1
        if result.stdout == expectedStdout:
            logger.success(f"Test passed '{testName}'.")
            passCount += 1
        else:
            logger.error(f"Test failed '{testName}' due to output mismatch.")
            failCount += 1
    logger.info(f"Passed: {passCount}. Failed: {failCount}. Total: {totalCount}.")
    if failCount > 0:
        exit(1)
