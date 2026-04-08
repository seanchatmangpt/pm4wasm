const handler = new (await import('./src/error-handler.ts')).ErrorHandler();

const code = 'A --> A';
const errors = handler.validate(code);
console.log('Initial errors:', errors);

const fixedCode = 'A --> B\nB --> C';
const fixedErrors = handler.validate(fixedCode);
console.log('Fixed code errors:', fixedErrors);
