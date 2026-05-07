import entry from 'libentry.so';
import { oauthManager } from '../oauth/OAuthManager';

@Entry
@Component
struct Index {
  @State logMessage: string = '';

  build() {
    Column() {
      Text('VFS APIs Test')
        .fontSize(24)
        .fontWeight(FontWeight.Bold)
        .padding({ top: 20, bottom: 10 })

      Divider()

      List({ space: 10 }) {
        ListItem() {
          Button('get at')
            .width('100%')
            .onClick(async () => {
              this.logMessage = 'Loading...';
              let at = await oauthManager.getAT()!!;
              console.log('auth = ' + at);
              this.logMessage = at as string;
              entry.setAt(at as string);
            })
        }

        ListItem() {
          Button('List Files')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/xxx/zzz';
                this.logMessage = 'list files test\n\n' + entry.listFiles(path);
              } catch (e) {
                this.logMessage = `List Files Error: ${e}`;
              }
            })
        }

        ListItem() {
          Button('Read File')
            .width('100%')
            .onClick(() => {
              this.logMessage = 'Loading...';
              try {
                const path = '/xxx/zzz/write_test.txt';
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
                const path = '/xxx/zzz/write_test.txt';
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
                const path = '/xxx/zzz/write_test.txt';
                const content = [72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33];
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
                const path = '/xxx/zzz/write_test.txt';
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
                const path = '/xxx/zzz/test_mk_dir';
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
                const path = '/xxx/zzz/write_test.txt';
                this.logMessage = 'stat file test\n\n' + entry.statFile(path);
              } catch (e) {
                this.logMessage = `Stat File Error: ${e}`;
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
