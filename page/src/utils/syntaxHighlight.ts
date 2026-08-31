// Pure TypeScript syntax tokenizer for Track and CLI code snippets

export function highlightTrack(code: string): string {
  // Escape HTML entities first, then apply span wrappers
  let escaped = code
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  // Regex patterns
  const stringRegex = /(&quot;.*?&quot;|".*?")/g;
  const numberRegex = /\b(0x[0-9a-fA-F]+|0b[01]+|\d+)\b/g;
  const keywordRegex = /\b(let|mut|fn|struct|enum|union|match|if|else|while|return|with|type|const|import|abort|as|macro|use)\b/g;
  const typeRegex = /\b(i8|i16|i32|i64|u8|u16|u32|u64|bool|void|ptr|Str|Vec|Option|Result|Token|Expr|Stmt)\b/g;
  const arrowRegex = /(-&gt;|=&gt;)/g;

  return escaped
    .split("\n")
    .map(line => {
      // If line is comment
      if (line.trim().startsWith("//")) {
        return `<span class="text-zinc-500 italic">${line}</span>`;
      }

      // Replace strings first by placeholder
      const strings: string[] = [];
      let tokenized = line.replace(stringRegex, match => {
        const id = `__STR_${strings.length}__`;
        strings.push(`<span class="text-emerald-400 font-medium">${match}</span>`);
        return id;
      });

      // Highlight keywords
      tokenized = tokenized.replace(keywordRegex, '<span class="text-amber-400 font-semibold">$1</span>');

      // Highlight types
      tokenized = tokenized.replace(typeRegex, '<span class="text-cyan-400 font-medium">$1</span>');

      // Highlight numbers
      tokenized = tokenized.replace(numberRegex, '<span class="text-amber-300">$1</span>');

      // Highlight arrows
      tokenized = tokenized.replace(arrowRegex, '<span class="text-rose-400 font-bold">$1</span>');

      // Function definitions: fn name(
      tokenized = tokenized.replace(/\bfn\s+([a-zA-Z0-9_]+)/g, '<span class="text-amber-400 font-semibold">fn</span> <span class="text-purple-400 font-semibold">$1</span>');

      // Variant paths: Color::Red, Value::Int
      tokenized = tokenized.replace(/([A-Z][a-zA-Z0-9_]*)::([A-Za-z0-9_]+)/g, '<span class="text-cyan-300 font-semibold">$1</span>::<span class="text-yellow-300 font-medium">$2</span>');

      // Restore strings
      strings.forEach((str, idx) => {
        tokenized = tokenized.replace(`__STR_${idx}__`, str);
      });

      return tokenized;
    })
    .join("\n");
}

export function highlightBash(code: string): string {
  let escaped = code
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  const lines = escaped.split("\n").map(line => {
    if (line.trim().startsWith("#")) {
      return `<span class="text-zinc-500 italic">${line}</span>`;
    }
    let tokenized = line;
    // Command prefixes
    tokenized = tokenized.replace(/\b(curl|yard|track|cargo|bash|git|cd|mkdir|cp|echo)\b/g, '<span class="text-amber-400 font-semibold">$1</span>');
    // Flags
    tokenized = tokenized.replace(/(--[a-zA-Z0-9_-]+|-[a-zA-Z0-9]+)/g, '<span class="text-cyan-400 font-mono">$1</span>');
    // Strings
    tokenized = tokenized.replace(/(&quot;.*?&quot;|".*?")/g, '<span class="text-emerald-400">$1</span>');
    return tokenized;
  });

  return lines.join("\n");
}
