import { pushCommon, PushExtensionAbility } from "@kit.PushKit";
import entry from 'libvfs_apis.so';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { deviceInfo } from '@kit.BasicServicesKit';

const LOG_DOMAIN: number = 0xFF00;
const LOG_TAG: string = 'VFS_PUSH';

const P2P_IDS_URL = 'http://81.71.29.250:28989';
const P2P_NAT_URL = 'https://natservice-drcn.platform.dbankcloud.cn:443/trs/v1/route';
const P2P_APP_ID = 'HiCloud';
const P2P_USER_ID = 'user123';
const P2P_ODID = deviceInfo.ODID;

export default class PushExtensionSampleAbility extends PushExtensionAbility {
  onReceiveMessage(payload: pushCommon.PushPayload): void {
    hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] onReceiveMessage called');
    let basePath =  '/data/storage/el2/base/files/taichu02';
    let workspacePath = '/usr10086/session100000';
    entry.setBasePath(basePath);
    entry.setWorkspace(workspacePath);
    // Extract message data from push payload.
    // Payload structure: {"data":"{...inner message...}", "header":{...}}
    const rawData = payload.data;
    if (!rawData || rawData.length === 0) {
      hilog.warn(LOG_DOMAIN, LOG_TAG, '[Push] empty payload data, skip');
      return;
    }

    hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] raw payload: %{public}s', rawData);

    // Parse outer JSON and extract the inner message from the "data" field
    let innerMessage: string;
    try {
      const outer = JSON.parse(rawData) as object;
      innerMessage = outer['data'] as string;
      if (!innerMessage || innerMessage.length === 0) {
        hilog.warn(LOG_DOMAIN, LOG_TAG, '[Push] no inner data field, using raw');
        innerMessage = rawData;
      }
    } catch (e) {
      hilog.warn(LOG_DOMAIN, LOG_TAG, '[Push] JSON.parse failed, using raw: %{public}s', String(e));
      innerMessage = rawData;
    }

    hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] inner message: %{public}s', innerMessage);

    // Step 1: Process the push message via Rust handlePushMessage
    let result: string;
    try {
      result = entry.handlePushMessage(innerMessage) as string;
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