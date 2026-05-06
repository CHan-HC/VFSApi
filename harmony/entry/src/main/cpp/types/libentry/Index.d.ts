export const setWorkspace: (path: string) => number;

export const listFiles: (at: string, path: string) => string;

export const readFile: (at: string, path: string) => string;

export const uploadFile: (at: string, path: string) => string;

export const writeFile: (at: string, path: string, content: number[]) => string;

export const rmFile: (at: string, path: string) => string;

export const mkDir: (at: string, path: string) => string;

export const statFile: (at: string, path: string) => string;
