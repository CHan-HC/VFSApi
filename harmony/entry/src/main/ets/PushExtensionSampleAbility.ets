import { pushCommon, PushExtensionAbility } from "@kit.PushKit";
import entry from 'libvfs_apis.so';
import { hilog } from '@kit.PerformanceAnalysisKit';

const LOG_DOMAIN: number = 0xFF00;
const LOG_TAG: string = 'VFS_PUSH';

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

    // Step 1: Process the push message via Rust handlePushMessage (mimics channel::on_message logic)
    let result: string;
    try {
      result = entry.handlePushMessage(pushData) as string;
      hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] handlePushMessage result: %{public}s', result);
    } catch (e) {
      hilog.error(LOG_DOMAIN, LOG_TAG, '[Push] handlePushMessage error: %{public}s', String(e));
      return;
    }

    // Step 2: Connect via P2P channel
    try {
      const idsUrl = 'http://81.71.29.250:28989';
      const natUrl = 'https://natservice-drcn.platform.dbankcloud.cn:443/trs/v1/route';
      const appId = 'HiCloud';
      const userId = 'user123';
      const odid = 'test_odid';
      const connectResult = entry.p2pConnect(idsUrl, natUrl, appId, userId, odid) as string;
      hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] P2P connect result: %{public}s', connectResult);
    } catch (e) {
      hilog.error(LOG_DOMAIN, LOG_TAG, '[Push] P2P connect error: %{public}s', String(e));
      return;
    }

    // Step 3: Send the processed result via P2P channel
    try {
      const sendResult = entry.p2pSendText(result) as string;
      hilog.info(LOG_DOMAIN, LOG_TAG, '[Push] P2P send result: %{public}s', sendResult);
    } catch (e) {
      hilog.error(LOG_DOMAIN, LOG_TAG, '[Push] P2P send error: %{public}s', String(e));
    }
  }
}