import type { Chapter } from '../types';

export const stdlib: Chapter = {
  id: 'stdlib',
  title: 'Standard Library',
  lessons: [
    {
      id: 'json',
      title: 'JSON',
      instructions: `## JSON Parsing & Serialization

Oxy includes a built-in \`json::\` module for working with JSON data.

\`\`\`
let parsed = json::parse(text);           // Result<Value, String>
let serialized = json::to_string(value);  // Result<String, String>
\`\`\`

\`json::parse()\` returns a \`Result\` — use \`if let Ok(v) = ...\` or \`match\` to extract the value.

Access JSON fields with bracket notation: \`v["key"]\` for objects, \`v[index]\` for arrays.

**Your task:** Parse a JSON string representing a person (\`{\\"name\\": \\"Alice\\", \\"age\\": 30}\`). Extract the name and age fields and print them.`,
      hints: [
        'Use \`json::parse(text)\` which returns \`Result<Value, String>\`.',
        'Match on the result: \`if let Ok(v) = json::parse(...) { ... }\`.',
        'Access fields: \`v["name"]\`, \`v["age"]\`.',
        'Use \`json::to_string(value)\` to serialize back to a JSON string.',
      ],
      initialCode: `fn main() {
    let text = "{\\"name\\": \\"Alice\\", \\"age\\": 30}";

    // TODO: parse the JSON string and extract "name" and "age"
    match json::parse(text) {
        Ok(v) => {
            println!("name: {}", v[___]);
            println!("age: {}", v[___]);
        },
        Err(e) => println!("parse error: {}", e),
    }

    // TODO: serialize a value back to JSON
    let data = vec![1, 2, 3];
    match json::to_string(data) {
        Ok(s) => println!("serialized: {}", s),
        Err(e) => println!("error: {}", e),
    }
}
`,
      testCode: `#[test] fn test_json_parse_object() {
    let text = "{\\"name\\": \\"Oxy\\", \\"year\\": 2024}";
    let result = json::parse(text);
    if let Ok(v) = result {
        assert_eq!(v["name"], "Oxy");
        assert_eq!(v["year"], 2024);
    } else {
        assert!(false);
    }
}

#[test] fn test_json_parse_array() {
    let text = "[1, 2, 3]";
    let result = json::parse(text);
    if let Ok(v) = result {
        assert_eq!(v[0], 1);
        assert_eq!(v[2], 3);
    } else {
        assert!(false);
    }
}

#[test] fn test_json_parse_nested() {
    let text = "{\\"person\\": {\\"name\\": \\"Bob\\", \\"scores\\": [10, 20]}}";
    let result = json::parse(text);
    if let Ok(v) = result {
        assert_eq!(v["person"]["name"], "Bob");
        assert_eq!(v["person"]["scores"][1], 20);
    } else {
        assert!(false);
    }
}

#[test] fn test_json_parse_invalid() {
    let result = json::parse("not valid json");
    assert!(result.is_err());
}

#[test] fn test_json_to_string() {
    let data = vec![true, false, true];
    let result = json::to_string(data);
    if let Ok(s) = result {
        assert!(s.contains("true"));
        assert!(s.contains("false"));
    } else {
        assert!(false);
    }
}

#[test] fn test_json_roundtrip() {
    let text = "{\\"x\\": 10, \\"y\\": 20}";
    let parsed = json::parse(text);
    if let Ok(v) = parsed {
        let serialized = json::to_string(v);
        assert!(serialized.is_ok());
    } else {
        assert!(false);
    }
}
`,
    },
    {
      id: 'io',
      title: 'File I/O',
      instructions: `## File I/O

The \`std::fs::\` module provides filesystem operations that return \`Result\`:

\`\`\`
std::fs::read_to_string(path)   // Result<String, String>
std::fs::write(path, content)   // Result<(), String>
std::fs::exists(path)           // bool
\`\`\`

All file operations return \`Result\` types — use \`if let Ok\` or \`match\` to handle potential errors.

**Your task:** Write a "notes" program. Create a file at \`/tmp/oxy-notes.txt\` with the content \`"Hello from Oxy!"\`. Then read it back and print the contents. Finally check that the file exists.`,
      hints: [
        'Write the file: \`std::fs::write(path, content)\`.',
        'Read the file: \`std::fs::read_to_string(path)\`.',
        'Check existence: \`std::fs::exists(path)\`.',
        'Use \`/tmp/oxy-notes.txt\` as the path for your test file.',
      ],
      initialCode: `fn main() {
    let path = "/tmp/oxy-notes.txt";

    // TODO: write "Hello from Oxy!" to the file
    ___;

    // TODO: read the file back and print its contents
    match std::fs::read_to_string(path) {
        Ok(content) => println!("file contains: {}", content),
        Err(e) => println!("error reading: {}", e),
    }

    // TODO: check if the file exists
    if std::fs::exists(path) {
        println!("file exists!");
    }
}
`,
      testCode: `#[test] fn test_fs_write_and_read() {
    let path = "/tmp/oxy-test-write.txt";
    let write_result = std::fs::write(path, "test content");
    assert!(write_result.is_ok());
    let read_result = std::fs::read_to_string(path);
    if let Ok(content) = read_result {
        assert_eq!(content, "test content");
    } else {
        assert!(false);
    }
}

#[test] fn test_fs_exists() {
    let path = "/tmp/oxy-test-exists.txt";
    std::fs::write(path, "hello");
    assert!(std::fs::exists(path));
    assert!(!std::fs::exists("/tmp/nonexistent_file_xyz"));
}

#[test] fn test_fs_read_nonexistent() {
    let result = std::fs::read_to_string("/tmp/nonexistent_file_xyz");
    assert!(result.is_err());
}

#[test] fn test_fs_overwrite() {
    let path = "/tmp/oxy-test-overwrite.txt";
    std::fs::write(path, "first");
    std::fs::write(path, "second");
    let content = std::fs::read_to_string(path);
    if let Ok(c) = content {
        assert_eq!(c, "second");
    } else {
        assert!(false);
    }
}
`,
    },
    {
      id: 'math',
      title: 'Math',
      instructions: `## Math Module

\`math::\` provides mathematical functions:

\`\`\`
math::sqrt(x)      // square root (float)
math::pow(x, y)    // x raised to y (float)
math::abs(x)       // absolute value (int or float)
math::max(a, b)    // larger of two numbers
math::min(a, b)    // smaller of two numbers
math::PI           // 3.141592653589793
math::E            // 2.718281828459045
\`\`\`

**Your task:** Implement \`hypotenuse\` that takes two legs of a right triangle and returns the hypotenuse using \`math::sqrt\` and \`math::pow\`. Also implement \`clamp\` that constrains a value within a range using \`math::max\` and \`math::min\`.`,
      hints: [
        'Hypotenuse: \`sqrt(a^2 + b^2)\`. Use \`math::pow(a, 2.0)\` and \`math::sqrt()\`.',
        'Clamp: \`math::max(lower, math::min(upper, value))\`.',
        'For absolute value: \`math::abs(x)\` works on both int and float.',
      ],
      initialCode: `fn hypotenuse(a: float, b: float) -> float {
    // TODO: return sqrt(a^2 + b^2)
    ___
}

fn clamp(value: int, lower: int, upper: int) -> int {
    // TODO: constrain value between lower and upper (inclusive)
    ___
}

fn main() {
    println!("hypotenuse(3, 4) = {}", hypotenuse(3.0, 4.0));
    println!("clamp(15, 0, 10) = {}", clamp(15, 0, 10));
    println!("clamp(-5, 0, 10) = {}", clamp(-5, 0, 10));
    println!("clamp(7, 0, 10) = {}", clamp(7, 0, 10));
    println!("abs(-42) = {}", math::abs(-42));
    println!("PI = {}", math::PI);
}
`,
      testCode: `#[test] fn test_math_sqrt() {
    let result = math::sqrt(16.0);
    assert_eq!(math::abs(result - 4.0) < 0.0001, true);
}

#[test] fn test_math_pow() {
    let result = math::pow(2.0, 10.0);
    assert_eq!(math::abs(result - 1024.0) < 0.0001, true);
}

#[test] fn test_math_abs_int() {
    assert_eq!(math::abs(-42), 42);
    assert_eq!(math::abs(0), 0);
    assert_eq!(math::abs(99), 99);
}

#[test] fn test_math_abs_float() {
    assert!(math::abs(-3.14) > 3.13);
}

#[test] fn test_math_max() {
    assert_eq!(math::max(3, 7), 7);
    assert_eq!(math::max(-1, -5), -1);
    assert_eq!(math::max(0, 0), 0);
}

#[test] fn test_math_min() {
    assert_eq!(math::min(3, 7), 3);
    assert_eq!(math::min(-1, -5), -5);
}

#[test] fn test_hypotenuse() {
    let h = hypotenuse(3.0, 4.0);
    assert!(math::abs(h - 5.0) < 0.0001);
}

#[test] fn test_clamp() {
    assert_eq!(clamp(15, 0, 10), 10);
    assert_eq!(clamp(-5, 0, 10), 0);
    assert_eq!(clamp(7, 0, 10), 7);
    assert_eq!(clamp(0, 0, 10), 0);
    assert_eq!(clamp(10, 0, 10), 10);
}
`,
    },
    {
      id: 'process',
      title: 'Process',
      instructions: `## Process Execution

The \`std::process::\` module lets you run external commands:

\`\`\`
std::process::command(program)                    // Result — run with no args
std::process::command_with_args(program, args)    // Result — run with args
\`\`\`

The return value is \`Result<CommandOutput, String>\` where \`CommandOutput\` has:
- \`stdout\` — captured standard output
- \`stderr\` — captured standard error
- \`status\` — exit code (int)
- \`success\` — whether the command succeeded (bool)

**Your task:** Run \`echo\` with arguments \`["Hello", "from", "Oxy"]\` using \`std::process::command_with_args\`. Print the captured stdout and whether it succeeded.`,
      hints: [
        'Use \`std::process::command_with_args("echo", vec!["arg1", "arg2"])\`.',
        'Check the result with \`if let Ok(output) = result\`.',
        'Access captured output: \`output.stdout\`.',
        'Access success field: \`output.success\`.',
      ],
      initialCode: `fn main() {
    // TODO: run "echo" with arguments vec!["Hello", "from", "Oxy"]
    let result = ___;

    match result {
        Ok(output) => {
            println!("stdout: {}", output.stdout.trim());
            println!("success: {}", output.success);
            println!("status: {}", output.status);
        },
        Err(e) => println!("error: {}", e),
    }
}
`,
      testCode: `#[test] fn test_process_command_echo() {
    let result = std::process::command("echo");
    if let Ok(output) = result {
        assert!(output.success);
    } else {
        assert!(false);
    }
}

#[test] fn test_process_command_with_args() {
    let result = std::process::command_with_args("echo", vec!["hello", "world"]);
    if let Ok(output) = result {
        let trimmed = output.stdout.trim();
        assert_eq!(trimmed, "hello world");
    } else {
        assert!(false);
    }
}

#[test] fn test_process_command_true() {
    let result = std::process::command("true");
    if let Ok(output) = result {
        assert!(output.success);
        assert_eq!(output.status, 0);
    } else {
        assert!(false);
    }
}

#[test] fn test_process_command_false() {
    let result = std::process::command("false");
    if let Ok(output) = result {
        assert!(!output.success);
        assert_eq!(output.status, 1);
    } else {
        assert!(false);
    }
}

#[test] fn test_process_nonexistent() {
    let result = std::process::command("nonexistent_program_xyz_123");
    assert!(result.is_err());
}
`,
    },
    {
      id: 'putting-together',
      title: 'Putting It All Together',
      instructions: `## Putting It All Together

Now combine multiple stdlib modules in a single program.

Your task: Build a program that:

1. Defines a JSON string with a list of items
2. Parses the JSON
3. Writes the parsed data to a file
4. Reads the file back
5. Uses \`math::\` to compute a value from the data
6. Runs a command to confirm the file was written

This demonstrates how the stdlib modules work together in a real-world scenario.`,
      hints: [
        'Parse JSON with \`json::parse()\`.',
        'Serialize with \`json::to_string()\` and write with \`std::fs::write()\`.',
        'Read back with \`std::fs::read_to_string()\`.',
        'Use \`math::sqrt\`, \`math::pow\`, etc. for any calculations.',
        'Verify with \`std::process::command_with_args("cat", vec![path])\` or \`std::fs::exists()\`.',
      ],
      initialCode: `fn main() {
    let path = "/tmp/oxy-combined-demo.txt";

    // Step 1: JSON with a list of numbers
    let data = "{\\"numbers\\": [3, 4, 5], \\"label\\": \\"right triangle\\"}";

    // Step 2: Parse JSON
    let parsed = ___;
    if let Ok(v) = parsed {
        println!("parsed: label = {}", v[___]);
        let nums = v[___]; // get the numbers array

        // Step 3: Compute hypotenuse
        let a = nums[___];
        let b = nums[___];
        let c = math::sqrt(math::pow(a, 2.0) + math::pow(b, 2.0));
        println!("hypotenuse: {}", c);

        // Step 4: Write to file
        let content = "hypotenuse: " + c;
        ___;  // std::fs::write

        // Step 5: Read back
        match std::fs::read_to_string(path) {
            Ok(text) => println!("file says: {}", text.trim()),
            Err(e) => println!("read error: {}", e),
        }
    }
}
`,
      testCode: `#[test] fn test_combined_json_fs() {
    let path = "/tmp/oxy-combined-test.json";
    let data = "{\\"name\\": \\"test\\", \\"value\\": 42}";
    let parsed = json::parse(data);
    if let Ok(v) = parsed {
        let serialized = json::to_string(v);
        if let Ok(s) = serialized {
            let write_result = std::fs::write(path, s);
            assert!(write_result.is_ok());
            let read_back = std::fs::read_to_string(path);
            assert!(read_back.is_ok());
        } else {
            assert!(false);
        }
    } else {
        assert!(false);
    }
}

#[test] fn test_combined_math_process() {
    let result = std::process::command_with_args("echo", vec!["sqrt(16) = 4"]);
    if let Ok(output) = result {
        let sqrt_val = math::sqrt(16.0);
        assert!(math::abs(sqrt_val - 4.0) < 0.0001);
        assert!(output.success);
    } else {
        assert!(false);
    }
}

#[test] fn test_json_parse_and_access() {
    let data = "{\\"numbers\\": [1, 2, 3, 4, 5], \\"sum\\": 15}";
    let parsed = json::parse(data);
    if let Ok(v) = parsed {
        assert_eq!(v["sum"], 15);
        assert_eq!(v["numbers"][0], 1);
        assert_eq!(v["numbers"][4], 5);
    } else {
        assert!(false);
    }
}

#[test] fn test_fs_write_then_process_check() {
    let path = "/tmp/oxy-check.txt";
    std::fs::write(path, "present");
    assert!(std::fs::exists(path));
    let read = std::fs::read_to_string(path);
    if let Ok(content) = read {
        assert_eq!(content, "present");
    } else {
        assert!(false);
    }
}
`,
    },
  ],
};
