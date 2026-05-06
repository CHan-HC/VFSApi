export const setWorkspace: (path: string) => number;

export const setAt: (at: string) => number;

export const listFiles: (path: string) => string;

export const readFile: (path: string) => string;

export const uploadFile: (path: string) => string;

export const writeFile: (path: string, content: number[]) => string;

export const rmFile: (path: string) => string;

export const mkDir: (path: string) => string;

export const statFile: (path: string) => string;
