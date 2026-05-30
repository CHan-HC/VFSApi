import { pushCommon, pushService, PushExtensionAbility } from "@kit.PushKit";
import entry from 'libvfs_apis.so';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { deviceInfo } from '@kit.BasicServicesKit';

const LOG_DOMAIN: number = 0xFF00;
const LOG_TAG: string = 'VFS_PUSH';

const P2P_IDS_URL = 'http://81.71.29.250:28989';
const P2P_NAT_URL = 'https://natservice-drcn.platform.dbankcloud.cn:443/trs/v1/route';
const P2P_NAT_TOKEN_URL = "http://81.71.29.250:33666/genToken";
const P2P_APP_ID = 'HiCloud';
const P2P_USER_ID = 'user123';
const P2P_ODID = deviceInfo.ODID;

export default class PushExtensionSampleAbility extends PushExtensionAbility {
  onReceiveMessage(payload: pushCommon.PushPayload): void {
    hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] onReceiveMessage called');
    entry.setBasePath('/data/storage/el2/base/files/taichu02');
    entry.setWorkspace('/usr10086/session100000');

    // Extract inner message from push payload: {"data":"{...}", "header":{...}}
    const rawData = payload.data;
    if (!rawData || rawData.length === 0) {
      hilog.warn(LOG_DOMAIN, LOG_TAG, '[Push] empty payload data, skip');
      return;
    }
    hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] raw payload: %{public}s', rawData);

    let innerMessage: string;
    try {
      const outer = JSON.parse(rawData) as object;
      innerMessage = outer['data'] as string;
      if (!innerMessage || innerMessage.length === 0) {
        innerMessage = rawData;
      }
    } catch (e) {
      innerMessage = rawData;
    }
    hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] inner message: %{public}s', innerMessage);

    // Single Rust call: process message + P2P connect/send (all in Rust)
    pushService.getToken().then((pushToken: string) => {
      hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] token: %{public}s', pushToken);
      try {
        const result = entry.p2pHandlePushMessage(
          P2P_IDS_URL, P2P_NAT_URL, P2P_APP_ID, P2P_USER_ID, P2P_ODID, pushToken, innerMessage, P2P_NAT_TOKEN_URL
        ) as string;
        hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] p2pHandlePushMessage result: %{public}s', result);
      } catch (e) {
        hilog.error(LOG_DOMAIN, LOG_TAG, '[Push] p2pHandlePushMessage error: %{public}s', String(e));
      }
    }).catch((err: Error) => {
      hilog.error(LOG_DOMAIN, LOG_TAG, '[Push] getToken error: %{public}s', String(err));
    });
  }
}