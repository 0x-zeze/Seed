import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SRC_DIR = path.resolve(__dirname, "../src");

export interface ScriptValidationResult {
  file: string;
  hasMainDeclaration: boolean;
  hasTopLevelMainCall: boolean;
  hasNestedMainCall: boolean;
  isLastStatementMainCall: boolean;
  valid: boolean;
  errors: string[];
}

export function isMainCallExpression(node: ts.Node): boolean {
  if (ts.isCallExpression(node)) {
    // Direct: main()
    if (ts.isIdentifier(node.expression) && node.expression.text === "main") {
      return true;
    }
    // Chained: main().catch(...) or main().then(...)
    if (ts.isPropertyAccessExpression(node.expression)) {
      const propAccess = node.expression;
      if (
        (propAccess.name.text === "catch" || propAccess.name.text === "then") &&
        ts.isCallExpression(propAccess.expression)
      ) {
        const innerCall = propAccess.expression;
        if (ts.isIdentifier(innerCall.expression) && innerCall.expression.text === "main") {
          return true;
        }
      }
    }
  }
  return false;
}

export function validateRunnableScript(
  filePath: string,
  sourceContent?: string
): ScriptValidationResult {
  const content = sourceContent !== undefined ? sourceContent : fs.readFileSync(filePath, "utf-8");
  const fileName = path.basename(filePath);
  const sourceFile = ts.createSourceFile(fileName, content, ts.ScriptTarget.Latest, true);

  const errors: string[] = [];
  let hasMainDeclaration = false;
  let mainFunctionNode: ts.FunctionDeclaration | null = null;
  let hasTopLevelMainCall = false;
  let hasNestedMainCall = false;
  let isLastStatementMainCall = false;

  const statements = sourceFile.statements;

  for (const stmt of statements) {
    if (ts.isFunctionDeclaration(stmt) && stmt.name?.text === "main") {
      hasMainDeclaration = true;
      mainFunctionNode = stmt;
    }

    if (ts.isExpressionStatement(stmt) && isMainCallExpression(stmt.expression)) {
      hasTopLevelMainCall = true;
    }
  }

  if (statements.length > 0) {
    const lastStmt = statements[statements.length - 1];
    if (ts.isExpressionStatement(lastStmt) && isMainCallExpression(lastStmt.expression)) {
      isLastStatementMainCall = true;
    }
  }

  if (mainFunctionNode && mainFunctionNode.body) {
    function findNestedMainCall(node: ts.Node) {
      if (isMainCallExpression(node)) {
        hasNestedMainCall = true;
      }
      ts.forEachChild(node, findNestedMainCall);
    }
    findNestedMainCall(mainFunctionNode.body);
  }

  if (!hasMainDeclaration) {
    errors.push("Missing top-level 'async function main(): Promise<void>' declaration.");
  }
  if (!hasTopLevelMainCall) {
    errors.push("Missing top-level invocation of 'main().catch(...)'. Script will not execute when run!");
  }
  if (hasNestedMainCall) {
    errors.push("Corrupted control flow: 'main().catch(...)' was found INSIDE 'main()' function body!");
  }
  if (!isLastStatementMainCall) {
    errors.push("Control flow invariant broken: the final top-level statement must be 'main().catch(...)'.");
  }

  return {
    file: fileName,
    hasMainDeclaration,
    hasTopLevelMainCall,
    hasNestedMainCall,
    isLastStatementMainCall,
    valid: errors.length === 0,
    errors,
  };
}

function runSuite(): void {
  console.log("================================================================================");
  console.log(">>> Running Control Flow Guard & Entrypoint Validation Suite");
  console.log("================================================================================\n");

  const runnableScripts = ["store.ts", "instantiate.ts", "smoke.ts"];
  let allPassed = true;

  for (const scriptName of runnableScripts) {
    const fullPath = path.resolve(SRC_DIR, scriptName);
    const result = validateRunnableScript(fullPath);

    if (result.valid) {
      console.log(`[PASS] ${scriptName.padEnd(16)} -> Verified top-level main() entrypoint.`);
    } else {
      allPassed = false;
      console.error(`[FAIL] ${scriptName.padEnd(16)} -> Control flow corruption detected!`);
      for (const err of result.errors) {
        console.error(`       - ${err}`);
      }
    }
  }

  // Regression Test: verify that the corrupted instantiate.ts snippet is correctly caught and rejected
  console.log("\n>>> Testing Guard Regression Detection on Historical Corrupted Pattern...");
  const corruptedSnippet = `
import { handleRawChainError } from "./config.js";
async function main(): Promise<void> {
  try {
    console.log("step 1");
  } catch (err: any) {
    if (true) {
      console.log("ok");
    } else {
  // dangling else swallows the rest of main!
  console.log("step 2");
}
main().catch((err) => handleRawChainError("Unhandled", err));
      handleRawChainError("Denom", err);
    }
  }
`;
  const regressionResult = validateRunnableScript("corrupted_mock.ts", corruptedSnippet);
  if (!regressionResult.valid && regressionResult.hasNestedMainCall && !regressionResult.hasTopLevelMainCall) {
    console.log("[PASS] Historical corruption pattern was successfully intercepted by guard!");
  } else {
    allPassed = false;
    console.error("[FAIL] Guard failed to detect simulated control flow corruption!");
  }

  console.log("\n================================================================================");
  if (allPassed) {
    console.log(">>> ALL CONTROL FLOW GUARDS PASSED (0 ERRORS)");
    console.log("================================================================================\n");
    process.exit(0);
  } else {
    console.error(">>> CONTROL FLOW GUARDS FAILED");
    console.error("================================================================================\n");
    process.exit(1);
  }
}

if (process.argv[1] && process.argv[1].endsWith("verify-control-flow.test.ts")) {
  runSuite();
}
