import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { window } from '@kit.ArkUI';
import { pushService } from '@kit.PushKit';

export default class EntryAbility extends UIAbility {
  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    pushService.getToken().then(token => {
      console.info('token = ' + token);
    }).catch((err: BusinessError) => {
      //console.error(0x0000, 'testTag', 'Failed to get push token: %{public}d %{public}s', err.code, err.message);
      console.info('token = error.');
    })
  }
  onWindowStageCreate(windowStage: window.WindowStage): void {
    windowStage.loadContent('pages/Index', (err) => {
    });
  }
}