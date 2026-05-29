import entry from 'libvfs_apis.so';
import { oauthManager } from '../oauth/OAuthManager';
import { common } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { pushService } from '@kit.PushKit';
import { deviceInfo } from '@kit.BasicServicesKit';

const LOG_DOMAIN: number = 0xFF00;
const LOG_TAG: string = 'VFS_APIS_UI';

@Entry
@Component
struct Index {
  @State logMessage: string = '';
  @State cloudPath: string = '';

  build() {
    Column() {
      Text('VFS APIs Test')
        .fontSize(24)
        .fontWeight(FontWeight.Bold)
        .padding({ top: 20, bottom: 10 })

      Divider()

      Row() {
        TextInput({ placeholder: 'cloud path, e.g. /usr10086/session100000', text: this.cloudPath })
          .onChange((value: string) => {
            this.cloudPath = value;
          })
          .layoutWeight(1)
          .fontSize(12)
          .height(36)

        Button('Cloud Dir')
          .fontSize(12)
          .height(36)
          .onClick(() => {
            this.logMessage = 'Loading...';
            try {
              this.logMessage = 'cloud dir raw\n\n' + entry.listCloudRaw(this.cloudPath);
            } catch (e) {
              this.logMessage = `Cloud Dir Error: ${e}`;
            }
          })
      }
      .width('100%')
      .padding({ left: 10, right: 10, top: 6, bottom: 6 })

      Divider()

      List({ space: 10 }) {
        ListItem() {
          Button('get at')
            .width('100%')
            .onClick(async () => {
              this.logMessage = 'Loading...';
              const context = this.getUIContext().getHostContext() as common.UIAbilityContext;
              let basePath = context.getApplicationContext().filesDir + '/taichu02';
              let workspacePath = '/usr10086/session100000';
              entry.setBasePath(basePath);
              entry.setWorkspace(workspacePath);
              let at = await oauthManager.getAT()!!;
              console.log('auth = ' + at);
              //hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] get at success, token length: %{public}d', at.length);
              this.logMessage = at as string;
              entry.setAt(at as string);
            })
        }

        ListItem() {
          Button('List Dir')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/';
                this.logMessage = 'list dir test\n\n' + entry.listDir(path);
              } catch (e) {
                this.logMessage = `List Dir Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('Read File')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/0507.txt';
                this.logMessage = 'read file test\n\n' + entry.readFile(path);
              } catch (e) {
                this.logMessage = `Read File Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('Upload File')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/0507.txt';
                this.logMessage = 'upload file test\n\n' + entry.uploadFile(path);
              } catch (e) {
                this.logMessage = `Upload File Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('Write File')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/0507.txt';
                const content = [99, 87, 77, 66, 77, 77, 77, 87, 111, 114, 108, 100, 33];
                this.logMessage = 'write file test\n\n' + entry.writeFile(path, content);
              } catch (e) {
                this.logMessage = `Write File Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('Rm File')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/0507(1).txt';
                this.logMessage = 'rm file test\n\n' + entry.rmFile(path);
              } catch (e) {
                this.logMessage = `Rm File Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('MkDir')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/xxx/zzz/qqq';
                this.logMessage = 'mkdir test\n\n' + entry.mkDir(path);
              } catch (e) {
                this.logMessage = `MkDir Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('Stat File')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/0507.txt';
                this.logMessage = 'stat file test\n\n' + entry.statFile(path);
              } catch (e) {
                this.logMessage = `Stat File Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('startBinding')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                this.logMessage = 'startBinding test\n\n' + entry.bindServer();
              } catch (e) {
                this.logMessage = `startBinding Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('P2P Connect')
            .width('100%')
            .onClick(() => {
              hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Connect button clicked');
              this.logMessage = 'Running P2P Connect...';
              try {
                const idsUrl = 'http://81.71.29.250:28989'; // TODO: replace with real IDS URL
                const natUrl = 'https://natservice-drcn.platform.dbankcloud.cn:443/trs/v1/route'; // TODO: replace with real NAT URL
                const appId = 'HiCloud';
                const userId = 'user123';
                const odid = 'test_odid';
                const result = entry.p2pConnect(idsUrl, natUrl, appId, userId, odid, '') as string;
                hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Connect result: %{public}s', result);
                this.logMessage = 'P2P Connect\n\n' + result;
              } catch (e) {
                hilog.error(LOG_DOMAIN, LOG_TAG, '[UI] P2P Connect error: %{public}s', String(e));
                this.logMessage = `P2P Connect Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('P2P Register')
            .width('100%')
            .onClick(() => {
              hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Register button clicked');
              this.logMessage = 'Running P2P Register IDs...';
              try {
                const idsUrl = 'http://81.71.29.250:28989'; // TODO: replace with real IDS URL
                const natUrl = 'https://natservice-drcn.platform.dbankcloud.cn:443/trs/v1/route'; // TODO: replace with real NAT URL
                const appId = 'HiCloud';
                const userId = 'user123';
                const odid = 'test_odid';
                const extra = ''; // last parameter for register_ids, default empty
                const result = entry.p2pRegisterIds(idsUrl, natUrl, appId, userId, odid, extra) as string;
                hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Register result: %{public}s', result);
                this.logMessage = 'P2P Register IDs\n\n' + result;
              } catch (e) {
                hilog.error(LOG_DOMAIN, LOG_TAG, '[UI] P2P Register error: %{public}s', String(e));
                this.logMessage = `P2P Register Error: ${e}`;
              }
            })
        }




        ListItem() {
          Button('P2P ICE State')
            .width('100%')
            .onClick(() => {
              hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P ICE State button clicked');
              try {
                const result = entry.p2pIceState() as string;
                hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P ICE State: %{public}s', result);
                this.logMessage = 'P2P ICE State: ' + result;
              } catch (e) {
                hilog.error(LOG_DOMAIN, LOG_TAG, '[UI] P2P ICE State error: %{public}s', String(e));
                this.logMessage = `P2P ICE State Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('P2P Send Text')
            .width('100%')
            .onClick(() => {
              hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Send Text button clicked');
              try {
                const result = entry.p2pSendText('Hello from VFSApi P2P test') as string;
                hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Send Text: %{public}s', result);
                this.logMessage = 'P2P Send Text: ' + result;
              } catch (e) {
                hilog.error(LOG_DOMAIN, LOG_TAG, '[UI] P2P Send Text error: %{public}s', String(e));
                this.logMessage = `P2P Send Text Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('P2P Poll Msg')
            .width('100%')
            .onClick(() => {
              hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Poll Msg button clicked');
              try {
                const result = entry.p2pPollMessages() as string;
                hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Poll Msg: %{public}s', result);
                this.logMessage = 'P2P Received Messages:\n\n' + result;
              } catch (e) {
                hilog.error(LOG_DOMAIN, LOG_TAG, '[UI] P2P Poll Msg error: %{public}s', String(e));
                this.logMessage = `P2P Poll Msg Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('P2P Test')
            .width('100%')
            .onClick(() => {
              hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Test button clicked');
              this.logMessage = 'Running P2P Integration Test...';
              try {
                const result = entry.p2pTest() as string;
                hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Test result length: %{public}d', result.length);
                hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Test result:\n%{public}s', result);
                this.logMessage = 'P2P Integration Test\n\n' + result;
              } catch (e) {
                hilog.error(LOG_DOMAIN, LOG_TAG, '[UI] P2P Test error: %{public}s', String(e));
                this.logMessage = `P2P Test Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('P2P Close')
            .width('100%')
            .onClick(() => {
              hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Close button clicked');
              try {
                const result = entry.p2pClose() as string;
                hilog.info(LOG_DOMAIN, LOG_TAG, '[UI] P2P Close: %{public}s', result);
                this.logMessage = 'P2P Close: ' + result;
              } catch (e) {
                hilog.error(LOG_DOMAIN, LOG_TAG, '[UI] P2P Close error: %{public}s', String(e));
                this.logMessage = `P2P Close Error: ${e}`;
              }
            })
        }
      }
      .width('100%')
      .layoutWeight(1)
      .padding({ left: 20, right: 20, top: 10, bottom: 10 })

      Divider()

      Column() {
        Text('Result:')
          .fontSize(14)
          .fontWeight(FontWeight.Bold)
          .width('100%')
          .padding({ left: 20, right: 20, top: 10 })

        Scroll() {
          Text(this.logMessage)
            .fontSize(12)
            .fontColor('#333333')
            .width('100%')
            .padding({ left: 20, right: 20 })
        }
        .width('100%')
        .layoutWeight(1)
        .padding({ bottom: 10 })
      }
      .width('100%')
      .height('40%')
    }
    .width('100%')
    .height('100%')
  }
}
