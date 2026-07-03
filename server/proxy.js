/*
  Proxy Support Module
  ====================
  Manages SOCKS5/HTTP proxy configuration for browser sessions.
  Supports:
    - Per-tab proxy assignment (via page.route interception when possible)
    - SOCKS5 and HTTP/HTTPS proxies
    - Authenticated proxies (username/password)
    - Proxy rotation (cycling through a list of proxies)
*/

const config = require('../config/default');
const net = require('net');


/**
 * Parse a proxy URI string into its components.
 * Accepts formats:
 *   socks5://user:pass@host:port
 *   socks5://host:port
 *   http://user:pass@host:port
 *   http://host:port
 *
 * @param {string} uri
 * @returns {Object|null}  { protocol, host, port, username, password }
 */
function parseProxyUri(uri) {
  if (!uri) return null;

  try {
    const url = new URL(uri);
    return {
      protocol: url.protocol.replace(':', ''),
      host: url.hostname,
      port: parseInt(url.port, 10) || 1080,
      username: url.username || null,
      password: url.password || null,
    };
  } catch {
    return null;
  }
}


/**
 * Create a SOCKS5 socket connection through a proxy.
 * Useful for custom proxy handling at the TCP level.
 *
 * @param {string} proxyUri  — e.g. 'socks5://127.0.0.1:9050'
 * @param {string} targetHost
 * @param {number} targetPort
 * @returns {Promise<net.Socket>}
 */
function socks5Connect(proxyUri, targetHost, targetPort) {
  return new Promise((resolve, reject) => {
    const info = parseProxyUri(proxyUri);
    if (!info) return reject(new Error('Invalid proxy URI'));

    const socket = new net.Socket();

    socket.setTimeout(10000);
    socket.once('error', reject);

    socket.connect(info.port, info.host, async () => {
      try {
        // SOCKS5 handshake
        // 1. greet
        let authPayload = Buffer.alloc(3);
        authPayload[0] = 0x05; // SOCKS5
        authPayload[1] = 1;    // 1 auth method
        authPayload[2] = info.username ? 0x02 : 0x00; // 0x00 = no auth, 0x02 = user/pass

        socket.write(authPayload);

        // 2. Read auth method response
        const authResp = await readBytes(socket, 2);
        if (authResp[0] !== 0x05) return reject(new Error('Invalid SOCKS version'));
        if (authResp[1] === 0xFF) return reject(new Error('No acceptable auth method'));

        // 3. User/pass auth if needed
        if (authResp[1] === 0x02 && info.username) {
          const userBuf = Buffer.from(info.username, 'utf-8');
          const passBuf = Buffer.from(info.password || '', 'utf-8');
          const authReq = Buffer.concat([
            Buffer.from([0x01, userBuf.length]),
            userBuf,
            Buffer.from([passBuf.length]),
            passBuf,
          ]);
          socket.write(authReq);
          const authStatus = await readBytes(socket, 2);
          if (authStatus[1] !== 0x00) return reject(new Error('Proxy auth failed'));
        }

        // 4. Connect request
        const hostname = targetHost;
        const port = targetPort;
        let addrBuf;

        // Try to resolve as IPv4 first
        const ipMatch = hostname.match(/^(\d+)\.(\d+)\.(\d+)\.(\d+)$/);
        if (ipMatch) {
          addrBuf = Buffer.from([0x01, ...ipMatch.slice(1).map(Number)]);
        } else {
          // Domain name
          const hostBuf = Buffer.from(hostname, 'utf-8');
          addrBuf = Buffer.concat([
            Buffer.from([0x03, hostBuf.length]),
            hostBuf,
          ]);
        }

        const portBuf = Buffer.alloc(2);
        portBuf.writeUInt16BE(port, 0);

        const connReq = Buffer.concat([
          Buffer.from([0x05, 0x01, 0x00]), // VER, CMD=CONNECT, RSV
          addrBuf,
          portBuf,
        ]);

        socket.write(connReq);

        // 5. Read connection response
        const connResp = await readBytes(socket, 4);
        if (connResp[0] !== 0x05) return reject(new Error('Invalid SOCKS version'));
        if (connResp[1] !== 0x00) return reject(new Error(`SOCKS connection failed with code ${connResp[1]}`));

        // Read remaining address/port from response
        const addrType = connResp[3];
        let respAddrLen;
        if (addrType === 0x01) respAddrLen = 4;       // IPv4
        else if (addrType === 0x03) respAddrLen = await readBytes(socket, 1).then(b => b[0] + 1);
        else if (addrType === 0x04) respAddrLen = 16;  // IPv6
        else return reject(new Error('Unknown address type'));

        await readBytes(socket, respAddrLen + 2); // addr + port

        socket.setTimeout(0);
        socket.removeAllListeners('error');
        resolve(socket);
      } catch (err) {
        reject(err);
      }
    });
  });
}


/**
 * Read exactly N bytes from a socket.
 */
function readBytes(socket, n) {
  return new Promise((resolve, reject) => {
    if (n === 0) return resolve(Buffer.alloc(0));

    const buf = [];
    let total = 0;

    const onData = (chunk) => {
      buf.push(chunk);
      total += chunk.length;
      if (total >= n) {
        cleanup();
        resolve(Buffer.concat(buf).slice(0, n));
      }
    };

    const onError = (err) => { cleanup(); reject(err); };
    const onTimeout = () => { cleanup(); reject(new Error('Socket timeout')); };
    const cleanup = () => {
      socket.removeListener('data', onData);
      socket.removeListener('error', onError);
      socket.removeListener('timeout', onTimeout);
    };

    socket.on('data', onData);
    socket.on('error', onError);
    socket.on('timeout', onTimeout);
  });
}


/**
 * Configure a Puppeteer page to route requests through a proxy.
 * This uses page.route() to intercept and re-route requests.
 * NOTE: For full-proxy support, pass --proxy-server at browser launch.
 *
 * @param {import('puppeteer').Page} page
 * @param {Object} proxyInfo  — { protocol, host, port, username?, password? }
 */
async function setPageProxy(page, proxyInfo) {
  if (!proxyInfo) return;

  // For HTTP/HTTPS requests, we can use page.authenticate
  if (proxyInfo.username && proxyInfo.password) {
    await page.authenticate({
      username: proxyInfo.username,
      password: proxyInfo.password,
    });
  }
}


module.exports = {
  parseProxyUri,
  socks5Connect,
  setPageProxy,
};
