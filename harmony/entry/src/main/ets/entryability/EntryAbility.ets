import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { window } from '@kit.ArkUI';
import entry from 'libentry.so';
import { oauthManager } from '../oauth/OAuthManager';

export default class EntryAbility extends UIAbility {
  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    const workspacePath = this.context.getApplicationContext().filesDir + '/applicationdata';
    entry.setWorkspace(workspacePath);
    oauthManager.init(this.context);
    oauthManager.getAT().then((at) => {
      if (at) {
        entry.setAt(at);
      }
    });
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    windowStage.loadContent('pages/Index', (err) => {
    });
  }
}