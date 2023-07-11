import type { Project } from '$lib/api';
import { Terminal } from 'xterm';
import { CanvasAddon } from 'xterm-addon-canvas';
import { WebglAddon } from 'xterm-addon-webgl';
import { FitAddon } from 'xterm-addon-fit';
import { Unicode11Addon } from 'xterm-addon-unicode11';
import WebSocket, { type Message } from 'tauri-plugin-websocket-api';
import { dev } from '$app/environment';

const isWebgl2Supported = (() => {
	let isSupported = window.WebGL2RenderingContext ? undefined : false;
	return () => {
		if (isSupported === undefined) {
			const canvas = document.createElement('canvas');
			const gl = canvas.getContext('webgl2', { depth: false, antialias: false });
			isSupported = gl instanceof window.WebGL2RenderingContext;
		}
		return isSupported;
	};
})();

const uint8ArrayToNumbers = (array: Uint8Array) => {
	const numbers = [];
	for (let i = 0; i < array.length; i++) {
		numbers.push(array[i]);
	}
	return numbers;
};

const encodeString = (msg: string): Message => ({
	type: 'Binary',
	data: uint8ArrayToNumbers(new TextEncoder().encode(msg))
});

const userInputMessage = (data: string) => encodeString(`\x00${data}`);

const resizeMessage = (size: {
	rows: number;
	cols: number;
	pixel_width: number;
	pixel_height: number;
}) => encodeString(`\x01${JSON.stringify(size)}`);

const newSession = async (params: { project: Project }) => {
	const { project } = params;
	const port = dev ? 7702 : 7703;
	return WebSocket.connect(`ws://localhost:${port}/${project.id}`).then((conn) => {
		const sendMessage = (message: Message) => {
			conn.send(message).catch((e: any) => {
				console.error(`failed to send message to terminal: ${e}`);
			});
		};

		const term = new Terminal({
			cursorBlink: false,
			cursorStyle: 'block',
			fontSize: 13,
			rows: 24,
			cols: 80,
			allowProposedApi: true
		});
		const fitAddon = new FitAddon();

		term.loadAddon(new Unicode11Addon());
		term.loadAddon(fitAddon);
		if (isWebgl2Supported()) {
			term.loadAddon(new WebglAddon());
		} else {
			term.loadAddon(new CanvasAddon());
		}

		term.unicode.activeVersion = '11';

		const inputListener = term.onData((data: string) => sendMessage(userInputMessage(data)));

		conn.addListener((message) => {
			if (message.type === 'Binary') {
				term.write(new Uint8Array(message.data));
			}
		});

		conn.addListener((message) => {
			if (message.type === 'Close') {
				term.dispose();
				location.reload();
			}
		});

		return {
			resize: () => {
				fitAddon.fit();
				const size = fitAddon.proposeDimensions();
				if (!size) return;
				sendMessage(
					resizeMessage({
						cols: size.cols,
						rows: size.rows,
						pixel_width: 0,
						pixel_height: 0
					})
				);
			},
			destroy: () => {
				term.dispose();
				inputListener.dispose();
				conn.disconnect();
			},
			run: (command: string) => {
				sendMessage(userInputMessage(`${command}\n`));
			},
			bind: (target: HTMLElement) => {
				term.open(target);
				fitAddon.fit();
				term.focus();
			}
		};
	});
};

const sessions = new Map<string, ReturnType<typeof newSession>>();

export default (params: { project: Project }) => {
	const { project } = params;
	const session = sessions.get(project.id);
	if (session) return session;
	const s = newSession(params);
	sessions.set(project.id, s);
	return s;
};
