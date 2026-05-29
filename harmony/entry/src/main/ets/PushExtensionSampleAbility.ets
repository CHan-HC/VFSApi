import { pushCommon, PushExtensionAbility } from "@kit.PushKit";
import entry from 'libvfs_apis.so';
import { hilog } from '@kit.PerformanceAnalysisKit';

const LOG_DOMAIN: number = 0xFF00;
const LOG_TAG: string = 'VFS_PUSH';

const P2P_IDS_URL = 'http://81.71.29.250:28989';
const P2P_NAT_URL = 'https://natservice-drcn.platform.dbankcloud.cn:443/trs/v1/route';
const P2P_APP_ID = 'HiCloud';
const P2P_USER_ID = 'user123';
const P2P_ODID = 'test_odid';

export default class PushExtensionSampleAbility extends PushExtensionAbility {
  onReceiveMessage(payload: pushCommon.PushPayload): void {
    hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] onReceiveMessage called');

    // Extract message data from push payload
    const pushData = payload.data;
    if (!pushData || pushData.length === 0) {
      hilog.warn(LOG_DOMAIN, LOG_TAG, '[Push] empty payload data, skip');
      return;
    }

    hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] payload data: %{public}s', pushData);

    // Step 1: Process the push message via Rust handlePushMessage
    let result: string;
    try {
      result = entry.handlePushMessage(pushData) as string;
      hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] handlePushMessage result: %{public}s', result);
    } catch (e) {
      hilog.error(LOG_DOMAIN, LOG_TAG, '[Push] handlePushMessage error: %{public}s', String(e));
      return;
    }

    // Step 2: Connect via P2P (non-blocking).
    // When ICE reaches Connected/Completed, p2p_connect auto-sends `result` via P2P.
    try {
      const connectResult = entry.p2pConnect(P2P_IDS_URL, P2P_NAT_URL, P2P_APP_ID, P2P_USER_ID, P2P_ODID, result) as string;
      hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] P2P connect initiated: %{public}s', connectResult);
    } catch (e) {
      hilog.error(LOG_DOMAIN, LOG_TAG, '[Push] P2P connect error: %{public}s', String(e));
    }
  }
}