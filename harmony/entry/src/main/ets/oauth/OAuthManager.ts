import { common } from "@kit.AbilityKit";
import { http } from "@kit.NetworkKit";
import { util } from "@kit.ArkTS";
import { authentication } from "@kit.AccountKit";
import { hilog } from '@kit.PerformanceAnalysisKit';

const TAG = 'OAuthManager';
const DOMAIN = 0xFF00;

class OAuthConfig {
  static readonly CLIENT_ID: string = '请填写';
  static readonly CLIENT_SECRET: string = '请填写';
  static readonly OAuthUrl = 'https://oauth-login.cloud.huawei.com/oauth2/v3/token';
  static readonly SCOPES: string[] = [
    'https://www.huawei.com/auth/drive.appdata',
    'https://www.huawei.com/auth/drive.file'
  ];
  static readonly PERMISSIONS: string[] = ['serviceauthcode'];
  static readonly TIMEOUT: number = 10 * 1000;
  static readonly TOKEN_EXPIRE_BUFFER: number = 5 * 60 * 1000;
}

class OAuthManagerImpl {
  private _at: {
    access_token: string;
    expires_in: number;
    refresh_token?: string;
  } | null = null;
  private _tokenExpireTime: number = 0;
  private _context;

  init(context: common.UIAbilityContext) {
    this._context = context;
  }

  private isTokenValid(): boolean {
    if (!this._at || this._tokenExpireTime === 0) {
      return false;
    }
    return Date.now() < (this._tokenExpireTime - OAuthConfig.TOKEN_EXPIRE_BUFFER);
  }

  async getAT() {
    hilog.info(DOMAIN, TAG, 'getAT called');
    
    if (this._at && this.isTokenValid()) {
      hilog.info(DOMAIN, TAG, 'Returning cached valid token');
      return this._at.access_token;
    }
    
    if (this._at?.refresh_token) {
      hilog.info(DOMAIN, TAG, 'Attempting to refresh token...');
      try {
        await this.refreshToken(this._at.refresh_token);
        return this._at.access_token;
      } catch (error) {
        hilog.error(DOMAIN, TAG, 'Token refresh failed, will re-auth');
        this._at = null;
      }
    }

    hilog.info(DOMAIN, TAG, 'Starting new auth...');
    await this.auth();
    hilog.info(DOMAIN, TAG, `Auth completed, token: ${this._at?.access_token ? this._at.access_token.substring(0, 20) + '...' : 'null'}`);
    return this._at?.access_token;
  }

  private async auth() {
    return new Promise<string>((resolve, reject) => {
      hilog.info(DOMAIN, TAG, 'Starting auth request...');
      hilog.info(DOMAIN, TAG, `Scopes: ${OAuthConfig.SCOPES.join(', ')}`);
      
      const authRequest = new authentication.HuaweiIDProvider().createAuthorizationWithHuaweiIDRequest();
      authRequest.scopes = OAuthConfig.SCOPES;
      authRequest.permissions = OAuthConfig.PERMISSIONS;
      authRequest.forceAuthorization = true;
      authRequest.state = util.generateRandomUUID();
      
      try {
        const controller = new authentication.AuthenticationController(this._context);
        controller.executeRequest(authRequest).then(async (data) => {
          hilog.info(DOMAIN, TAG, 'Auth request successful, got response');
          const authResponse = data as authentication.AuthorizationWithHuaweiIDResponse;
          const authCredential = authResponse?.data;
          const authCode = authCredential?.authorizationCode;
          hilog.info(DOMAIN, TAG, `Got auth response: ${JSON.stringify(authResponse)}`);
          hilog.info(DOMAIN, TAG, `Got auth code: ${authCode ? authCode.substring(0, 10) + '...' : 'null'}`);
          
          let atContent = await this.getATByCode(authCode);
          hilog.info(DOMAIN, TAG, `Token response: ${atContent}`);
          
          this._at = JSON.parse(atContent);
          if (this._at && this._at.expires_in) {
            this._tokenExpireTime = Date.now() + (this._at.expires_in * 1000);
            hilog.info(DOMAIN, TAG, `Token expires in ${this._at.expires_in} seconds`);
          }
          resolve(this._at!.access_token);
        }).catch((err: Error) => {
          hilog.error(DOMAIN, TAG, `Auth request failed: ${err.message}`);
          reject(err);
        });
      } catch (err) {
        const error = err as Error;
        hilog.error(DOMAIN, TAG, `Auth exception: ${error.message}`);
        reject(error);
      }
    });
  }

  private async getATByCode(authCode: string) {
    return new Promise<string>((resolve, reject) => {
      hilog.info(DOMAIN, TAG, 'Exchanging auth code for token...');
      
      let request = http.createHttp();
      const url = `${OAuthConfig.OAuthUrl}?` +
        `client_id=${OAuthConfig.CLIENT_ID}` +
        `&client_secret=${OAuthConfig.CLIENT_SECRET}` +
        `&grant_type=authorization_code` +
        `&code=${encodeURIComponent(authCode)}`;
      
      hilog.info(DOMAIN, TAG, `Token URL: ${OAuthConfig.OAuthUrl}`);
      hilog.info(DOMAIN, TAG, `Client ID: ${OAuthConfig.CLIENT_ID}`);
      
      request.requestInStream(url, {
        method: http.RequestMethod.POST,
        connectTimeout: OAuthConfig.TIMEOUT,
        readTimeout: OAuthConfig.TIMEOUT,
        header: {
          'Content-Type': 'application/json'
        }
      }, (err) => {
        if (err) {
          hilog.error(DOMAIN, TAG, `Token request error: ${err.message || JSON.stringify(err)}`);
          reject(err);
        }
      })

      request.on('dataReceive', (data: ArrayBuffer) => {
        const decoder = new util.TextDecoder('utf-8');
        const content = decoder.decodeToString(new Uint8Array(data));
        hilog.info(DOMAIN, TAG, `Token response received: ${content.substring(0, 100)}...`);
        resolve(content);
      })
      
      request.on('dataEnd', () => {
        hilog.info(DOMAIN, TAG, 'Token request data end');
      });
    });
  }

  private async refreshToken(refreshToken: string) {
    return new Promise<void>((resolve, reject) => {
      let request = http.createHttp();
      const url = `${OAuthConfig.OAuthUrl}?` +
        `client_id=${OAuthConfig.CLIENT_ID}` +
        `&client_secret=${OAuthConfig.CLIENT_SECRET}` +
        `&grant_type=refresh_token` +
        `&refresh_token=${encodeURIComponent(refreshToken)}`;
      request.requestInStream(url, {
        method: http.RequestMethod.POST,
        connectTimeout: OAuthConfig.TIMEOUT,
        readTimeout: OAuthConfig.TIMEOUT,
        header: {
          'Content-Type': 'application/json'
        }
      }, (err) => {
        if (err) {
          reject(err);
        }
      })

      request.on('dataReceive', (data: ArrayBuffer) => {
        const decoder = new util.TextDecoder('utf-8');
        const content = decoder.decodeToString(new Uint8Array(data));
        console.log(`refresh token is ${content}`);
        try {
          this._at = JSON.parse(content);
          if (this._at && this._at.expires_in) {
            this._tokenExpireTime = Date.now() + (this._at.expires_in * 1000);
          }
          resolve();
        } catch (error) {
          reject(error);
        }
      })
    });
  }
}

export const oauthManager = new OAuthManagerImpl();