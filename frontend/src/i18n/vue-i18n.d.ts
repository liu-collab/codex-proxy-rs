import type { MessageSchema } from './messages'

declare module 'vue-i18n' {
  export interface DefineLocaleMessage extends MessageSchema {}
}
