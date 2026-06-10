import { execSync } from 'node:child_process';

const nightly = execSync('make --no-print-directory rust-toolchain-nightly').toString().trim();

export default {
    idl: 'idl.json',
    before: [],
    scripts: {
        js: {
            from: '@codama/renderers-js',
            args: ['clients/js', { kitImportStrategy: 'rootOnly', syncPackageJson: true }],
        },
        rust: {
            from: '@codama/renderers-rust',
            args: [
                'clients/rust',
                {
                    anchorTraits: false,
                    formatCode: true,
                    toolchain: `+${nightly}`,
                },
            ],
        },
    },
};
