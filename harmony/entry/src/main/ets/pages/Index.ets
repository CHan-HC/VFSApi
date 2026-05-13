import entry from 'libvfs_apis.so';
import { oauthManager } from '../oauth/OAuthManager';
import { common } from '@kit.AbilityKit';

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
              let workspacePath = basePath + '/usr10086/session100000';
              //entry.setBasePath(basePath);
              entry.setWorkspace(workspacePath);
              let at = await oauthManager.getAT()!!;
              console.log('auth = ' + at);
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
